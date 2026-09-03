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
pub async fn ask(
    host: &str,
    model: &str,
    prompt: &str,
    mut on_chunk: impl FnMut(&str),
) -> Result<String, String> {
    let url = format!("{}/api/generate", host.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let send = client.post(&url).json(&GenerateRequest {
        model,
        prompt,
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
                return Ok(answer);
            }
        }
    }

    Ok(answer)
}
