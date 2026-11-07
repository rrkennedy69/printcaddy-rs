use crate::document::Document;

/// Undo and redo for the whole document. One drag, or one typed number, is one step.
#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
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
        }
        self.redo.clear();
        self.open = true;
    }

    pub fn end(&mut self) {
        self.open = false;
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
        self.redo.push(std::mem::replace(doc, previous));
        true
    }

    pub fn redo(&mut self, doc: &mut Document) -> bool {
        self.end();
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(doc, next));
        true
    }
}
