use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::provider::{Action, Candidate, Provider};

/// 入力があれば常に「AIに聞く」候補を1件返すプロバイダ
pub struct LlmProvider;

impl Provider for LlmProvider {
    fn name(&self) -> &'static str {
        "AI"
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }

        vec![Candidate {
            label: format!("AI に「{q}」を聞く"),
            source: "AI",
            action: Action::AskLlm(q.to_string()),
        }]
    }
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Serialize)]
struct TavilySearchRequest<'a> {
    query: &'a str,
    max_results: u32,
    search_depth: &'a str,
}

#[derive(Deserialize)]
struct TavilySearchResult {
    title: String,
    url: String,
    content: String,
}

#[derive(Deserialize, Default)]
struct TavilySearchResponse {
    #[serde(default)]
    results: Vec<TavilySearchResult>,
}

/// Tavily で Web 検索する(接続確立のタイムアウトのみ、通常の HTTP リクエストなので短くてよい)
const TAVILY_TIMEOUT: Duration = Duration::from_secs(15);

async fn tavily_search(api_key: &str, query: &str) -> Result<Vec<TavilySearchResult>, String> {
    let client = reqwest::Client::new();
    let send = client
        .post("https://api.tavily.com/search")
        .bearer_auth(api_key)
        .json(&TavilySearchRequest {
            query,
            max_results: 5,
            search_depth: "basic",
        });

    let res = tokio::time::timeout(TAVILY_TIMEOUT, send.send())
        .await
        .map_err(|_| "Tavily からの応答がありません".to_string())?
        .map_err(|err| format!("Tavily に接続できませんでした: {err}"))?;

    if !res.status().is_success() {
        return Err(format!("Tavily がエラーを返しました: {}", res.status()));
    }

    let body: TavilySearchResponse = res
        .json()
        .await
        .map_err(|err| format!("Tavily の応答の解析に失敗しました: {err}"))?;

    Ok(body.results)
}

/// Web 検索結果を踏まえて質問するためのプロンプトを組み立てる
fn build_grounded_prompt(prompt: &str, results: &[TavilySearchResult]) -> String {
    let context = results
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {}\n{}\n出典: {}", i + 1, r.title, r.content, r.url))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "以下はWeb検索で見つかった情報です。これを参考にして、質問に日本語で答えてください。\n\
         検索結果に無い情報を推測で補わないでください。\n\n\
         [検索結果]\n{context}\n\n[質問]\n{prompt}"
    )
}

#[derive(Deserialize, Default)]
struct GenerateChunk {
    #[serde(default)]
    response: String,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    error: Option<String>,
}

/// 次のチャンクが届くまで、これ以上待たずにハングとみなす時間
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// 生成が終わらない場合に強制的に打ち切るまでの全体の上限
const OVERALL_TIMEOUT: Duration = Duration::from_secs(300);

/// Ollama の /api/generate に質問を送り、届いた分だけ on_chunk で通知しながら回答を取得する
///
/// 応答は長くなりうるため、待ち時間そのものは制限しない。代わりに、一定時間
/// 新しいデータが全く届かない場合だけハングとみなしてエラーにする
///
/// `tavily_api_key` が空でなければ、質問する前に Tavily で Web 検索し、その結果を
/// 踏まえて答えるようにプロンプトを組み立てる(検索結果は末尾に出典として付記する)。
/// キーが空、または検索自体に失敗した場合は、検索なしでそのまま質問する
pub async fn ask(
    host: &str,
    model: &str,
    tavily_api_key: &str,
    prompt: &str,
    mut on_chunk: impl FnMut(&str),
) -> Result<String, String> {
    let (llm_prompt, sources) = if tavily_api_key.trim().is_empty() {
        (prompt.to_string(), Vec::new())
    } else {
        match tavily_search(tavily_api_key, prompt).await {
            Ok(results) if !results.is_empty() => {
                let grounded = build_grounded_prompt(prompt, &results);
                (grounded, results)
            }
            _ => (prompt.to_string(), Vec::new()),
        }
    };

    let url = format!("{}/api/generate", host.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let send = client.post(&url).json(&GenerateRequest {
        model,
        prompt: &llm_prompt,
        stream: true,
    });

    let res = tokio::time::timeout(IDLE_TIMEOUT, send.send())
        .await
        .map_err(|_| "Ollama からの応答がありません".to_string())?
        .map_err(|err| format!("Ollama に接続できませんでした: {err}"))?;

    if !res.status().is_success() {
        return Err(format!("Ollama がエラーを返しました: {}", res.status()));
    }

    let mut stream = res.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut answer = String::new();
    let start = tokio::time::Instant::now();

    loop {
        if start.elapsed() > OVERALL_TIMEOUT {
            return Err("応答の生成が長時間終わらなかったため中断しました".to_string());
        }

        let chunk = match tokio::time::timeout(IDLE_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(bytes))) => bytes,
            Ok(Some(Err(err))) => return Err(format!("通信が中断されました: {err}")),
            Ok(None) => break,
            Err(_) => return Err("Ollama からの応答が止まりました".to_string()),
        };

        buf.extend_from_slice(&chunk);

        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = &line[..line.len() - 1];
            if line.is_empty() {
                continue;
            }

            let parsed: GenerateChunk = serde_json::from_slice(line)
                .map_err(|err| format!("応答の解析に失敗しました: {err}"))?;

            if let Some(message) = parsed.error {
                return Err(format!("Ollama がエラーを返しました: {message}"));
            }

            if !parsed.response.is_empty() {
                answer.push_str(&parsed.response);
                on_chunk(&answer);
            }

            if parsed.done {
                append_sources(&mut answer, &sources);
                on_chunk(&answer);
                return Ok(answer);
            }
        }
    }

    append_sources(&mut answer, &sources);
    on_chunk(&answer);
    Ok(answer)
}

/// 検索結果を出典として回答の末尾に付記する(検索を使っていなければ何もしない)
fn append_sources(answer: &mut String, sources: &[TavilySearchResult]) {
    if sources.is_empty() {
        return;
    }
    answer.push_str("\n\n参照元:\n");
    for s in sources {
        answer.push_str(&format!("- {}\n  {}\n", s.title, s.url));
    }
}
