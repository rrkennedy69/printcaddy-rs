use crate::document::Document;

/// Undo and redo for the whole document. One drag, or one typed number, is one step.
#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
    /// What each undo snapshot was left behind by. Same length as `undo` once the step closes.
    undo_labels: Vec<String>,
    redo_labels: Vec<String>,
    pending: String,
    /// A drag or a number field already stored the document from before the edit.
    open: bool,
}

impl History {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Remember `doc` as the state to return to, then clear redo. A second call
    /// while the edit is still open does not store another copy.
    pub fn begin(&mut self, doc: &Document) {
        if self.open {
            return;
        }
        self.undo.push(doc.clone());
        if self.undo.len() > 80 {
            self.undo.remove(0);
            if !self.undo_labels.is_empty() {
                self.undo_labels.remove(0);
            }
        }
        self.redo.clear();
        self.redo_labels.clear();
        self.open = true;
    }

    /// Name for the step that is open. The history list shows it.
    pub fn note(&mut self, label: &str) {
        self.pending = label.to_string();
    }

    pub fn end(&mut self) {
        if self.open {
            let label = if self.pending.is_empty() {
                "Change".to_string()
            } else {
                std::mem::take(&mut self.pending)
            };
            self.undo_labels.push(label);
        }
        self.open = false;
    }

    /// Past steps, oldest first, then the future steps that redo would restore.
    pub fn labels(&self) -> (Vec<String>, Vec<String>) {
        let mut future = self.redo_labels.clone();
        future.reverse();
        (self.undo_labels.clone(), future)
    }

    /// `past` counts closed steps from the oldest. The length of the past list is "now".
    pub fn jump(&mut self, doc: &mut Document, past: usize) -> bool {
        self.end();
        let mut moved = false;
        while self.undo.len() > past {
            if !self.undo(doc) {
                break;
            }
            moved = true;
        }
        while self.undo.len() < past {
            if !self.redo(doc) {
                break;
            }
            moved = true;
        }
        moved
    }

    /// Drop the open checkpoint, used when a second finger puts a move back.
    pub fn cancel(&mut self) {
        if self.open {
            self.undo.pop();
            self.open = false;
        }
    }

    pub fn undo(&mut self, doc: &mut Document) -> bool {
        self.end();
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        let label = self.undo_labels.pop().unwrap_or_else(|| "Change".into());
        self.redo.push(std::mem::replace(doc, previous));
        self.redo_labels.push(label);
        true
    }

    pub fn redo(&mut self, doc: &mut Document) -> bool {
        self.end();
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let label = self.redo_labels.pop().unwrap_or_else(|| "Change".into());
        self.undo.push(std::mem::replace(doc, next));
        self.undo_labels.push(label);
        true
    }
}
