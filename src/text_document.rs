use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

#[derive(Debug)]
pub struct FullTextDocument {
    language_id: String,
    version: i32,
    content: String,
    line_offsets: Vec<u32>,
}

fn computed_line_offsets(text: &str, is_at_line_start: bool, text_offset: Option<u32>) -> Vec<u32> {
    let text_offset = text_offset.unwrap_or(0);
    let mut line_offsets = if is_at_line_start {
        vec![text_offset]
    } else {
        vec![]
    };

    for (idx, char) in text.char_indices() {
        let idx: u32 = idx
            .try_into()
            .expect("The length of the text involved in the calculation is too long");
        if char == '\n' || char == '\r' {
            line_offsets.push(text_offset + idx + 1);
        }
    }

    line_offsets
}

impl FullTextDocument {
    pub fn new(language_id: String, version: i32, content: String) -> Self {
        let line_offsets = computed_line_offsets(&content, true, None);
        Self {
            language_id,
            version,
            content,
            line_offsets,
        }
    }

    pub fn update(&mut self, changes: &[TextDocumentContentChangeEvent], version: i32) {
        for change in changes {
            let TextDocumentContentChangeEvent { range, text, .. } = change;
            match range {
                Some(range) => {
                    // update content
                    let Range { start, end } = range;
                    let (start_offset, end_offset) = (self.offset_at(*start), self.offset_at(*end));
                    assert!(start_offset <= end_offset);
                    let (start_slice, end_slice) = (
                        self.content.get(0..start_offset as usize).unwrap_or(""),
                        self.content.get(end_offset as usize..).unwrap_or(""),
                    );
                    self.content = start_slice
                        .chars()
                        .chain(text.chars())
                        .chain(end_slice.chars())
                        .collect();

                    let (start_line, end_line) = (start.line, end.line);
                    assert!(start_line <= end_line);
                    let added_line_offsets =
                        computed_line_offsets(&text, false, Some(start_offset));

                    self.line_offsets = self
                        .line_offsets
                        .as_slice()
                        .get(0..(start_line + 1) as usize)
                        .unwrap_or(&[])
                        .iter()
                        .chain(added_line_offsets.iter())
                        .chain(
                            self.line_offsets
                                .as_slice()
                                .get((end_line + 1) as usize..)
                                .unwrap_or(&[]),
                        )
                        .map(|&x| x)
                        .collect::<Vec<_>>();

                    dbg!(&self.line_offsets);
                    let diff =
                        (text.len() as i32).saturating_sub_unsigned(end_offset - start_offset);
                    if diff != 0 {
                        let (start, end) = (
                            start_line + 1 + added_line_offsets.len() as u32,
                            self.line_count(),
                        );
                        for i in start..end {
                            self.line_offsets[i as usize] =
                                self.line_offsets[i as usize].saturating_add_signed(diff);
                        }
                    }
                }
                None => {
                    // Full Text
                    // update line_offsets
                    self.line_offsets = computed_line_offsets(&text, true, None);

                    // update content
                    self.content = text.to_owned();
                }
            }
        }

        self.version = version;
    }

    /// Document's language id
    pub fn language_id(&self) -> &str {
        &self.language_id
    }

    /// Document's version
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Get document content
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```
    /// use lsp_textdocument::FullTextDocument;
    /// use lsp_types::{Range, Position};
    ///
    /// let text_documents = FullTextDocument::new("plain_text".to_string(), 1, "hello rust!".to_string());
    ///
    /// // get document all content
    /// let content = text_documents.get_content(None);
    /// assert_eq!(content, "hello rust!");
    ///
    /// // get document specify content by range
    /// let (start, end) = (Position::new(0, 1), Position::new(0, 9));
    /// let range = Range::new(start, end);
    /// let sub_content = text_documents.get_content(Some(range));
    /// assert_eq!(sub_content, "ello rus");
    /// ```
    pub fn get_content(&self, range: Option<Range>) -> &str {
        match range {
            Some(Range { start, end }) => {
                let start = self.offset_at(start);
                let end = self.offset_at(end).min(self.content_len());
                self.content.get(start as usize..end as usize).unwrap()
            }
            None => &self.content,
        }
    }

    /// A amount of document content line
    pub fn line_count(&self) -> u32 {
        self.line_offsets
            .len()
            .try_into()
            .expect("The number of lines of text passed in is too long")
    }

    /// The len of the document content
    pub fn content_len(&self) -> u32 {
        self.content
            .chars()
            .count()
            .try_into()
            .expect("The length of the text passed in is too long")
    }

    /// Converts a zero-based offset to a position
    pub fn position_at(&self, offset: u32) -> Position {
        let offset = offset.min(self.content_len());
        let (mut low, mut high) = (0, self.line_count());
        if high - low == 1 {
            // only one line
            return Position {
                line: low,
                character: offset,
            };
        }
        while low < high {
            let mid = (low + high).div_floor(2);
            if offset
                > *self
                    .line_offsets
                    .get(mid as usize)
                    .expect("Unknown mid value")
            {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        let line = low - 1;

        Position {
            line,
            character: offset - self.line_offsets[line as usize],
        }
    }

    /// Converts a position to a zero-based offset
    pub fn offset_at(&self, position: Position) -> u32 {
        let Position { line, character } = position;
        match self.line_offsets.get(line as usize) {
            Some(&line_offset) => (character + line_offset).min(
                if let Some(&next_line_offset) = self.line_offsets.get((line + 1) as usize) {
                    next_line_offset
                } else {
                    self.content_len()
                },
            ),
            None => {
                if line >= self.line_count() {
                    self.content_len()
                } else {
                    0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_text_document() -> FullTextDocument {
        FullTextDocument::new("js".to_string(), 2, "he\nllo\nworld".to_string())
    }

    #[test]
    fn test_offset_at() {
        let text_document = full_text_document();

        let offset = text_document.offset_at(Position {
            line: 1,
            character: 1,
        });
        assert_eq!(offset, 4);

        let offset = text_document.offset_at(Position {
            line: 2,
            character: 3,
        });
        assert_eq!(offset, 10);
    }

    #[test]
    fn test_position_at() {
        let text_document = full_text_document();

        let position = text_document.position_at(5);
        assert_eq!(
            position,
            Position {
                line: 1,
                character: 2
            }
        );

        let position = text_document.position_at(11);
        assert_eq!(
            position,
            Position {
                line: 2,
                character: 4,
            }
        )
    }

    #[test]
    fn test_get_content() {
        let text_document = full_text_document();

        let start = Position {
            line: 0,
            character: 0,
        };
        let end = Position {
            line: 1,
            character: 2,
        };
        let range = Range { start, end };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "he\nll");

        let end = Position {
            line: 100,
            character: 100,
        };
        let range = Range { start, end };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "he\nllo\nworld");

        let range = Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 2,
                character: 3,
            },
        };
        let content = text_document.get_content(Some(range));
        assert_eq!(content, "llo\nwor");
    }

    #[test]
    fn test_update_full_content() {
        let mut text_document = full_text_document();
        let new_text = "hello\n js!";

        text_document.update(
            &[TextDocumentContentChangeEvent {
                text: new_text.to_string(),
                range: None,
                range_length: None,
            }],
            1,
        );

        assert_eq!(&text_document.content, new_text);
        assert_eq!(text_document.line_offsets, vec![0, 6]);
    }

    #[test]
    fn test_update_part_content() {
        let mut text_document = full_text_document();
        assert_eq!(text_document.version(), 2);
        let new_text = String::from("xx\ny");
        let range = Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 3,
            },
        };
        text_document.update(
            &[TextDocumentContentChangeEvent {
                range: Some(range),
                range_length: None,
                text: new_text,
            }],
            1,
        );

        assert_eq!(&text_document.content, "he\nxx\ny\nworld");
        assert_eq!(text_document.line_offsets, vec![0, 3, 6, 8]);
        assert_eq!(text_document.version(), 1)
    }
}
