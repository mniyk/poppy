use crate::provider::{Action, Candidate, Provider};
use crate::todos::SharedTodos;

pub struct TodoProvider {
    todos: SharedTodos,
}

impl TodoProvider {
    pub fn new(todos: SharedTodos) -> Self {
        Self { todos }
    }
}

impl Provider for TodoProvider {
    fn name(&self) -> &'static str {
        "Todo"
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim();
        let ql = q.to_lowercase();
        let mut list = Vec::new();

        if !q.is_empty() {
            list.push(Candidate {
                label: format!("「{q}」を TODO に追加"),
                source: "Todo",
                action: Action::AddTodo(q.to_string()),
            });
        }

        for t in self
            .todos
            .borrow()
            .iter()
            .filter(|t| q.is_empty() || t.text.to_lowercase().contains(&ql))
        {
            list.push(Candidate {
                label: format!("TODO の「{}」を完了にする", t.text),
                source: "Todo",
                action: Action::CompleteTodo(t.id),
            });
        }

        list
    }
}
