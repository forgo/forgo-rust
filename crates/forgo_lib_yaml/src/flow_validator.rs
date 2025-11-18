// // crates/forgo_lib_yaml/src/flow_validator.rs
// //! Flow collection validation for YAML 1.2.2 compliance.
// //!
// //! This module implements validation for flow sequences `[...]` and flow mappings `{...}`:
// //! - Proper delimiter matching (brackets and braces)
// //! - Comma requirements between elements
// //! - No trailing commas
// //! - Colon requirements in mappings

// use crate::ast::Error;
// use crate::lexer::Tok;

// // #[allow(dead_code)]
// // #[derive(Debug, Clone, Copy, PartialEq)]
// // enum FlowDelim {
// //     Bracket,  // [
// //     Brace,    // {
// // }

// /// Flow collection validator
// ///
// /// Validates flow syntax according to YAML 1.2.2 spec:
// /// - Matched delimiters
// /// - Proper comma placement
// /// - Colon requirements in mappings
// #[allow(dead_code)]
// pub struct FlowValidator {
//     stack: Vec<FlowDelim>,
// }

// #[allow(dead_code)]
// impl FlowValidator {
//     pub fn new() -> Self {
//         Self { stack: Vec::new() }
//     }

//     /// Check if we're inside a flow collection
//     pub fn in_flow(&self) -> bool {
//         !self.stack.is_empty()
//     }

//     /// Open a flow sequence `[`
//     pub fn open_bracket(&mut self) {
//         self.stack.push(FlowDelim::Bracket);
//     }

//     /// Open a flow mapping `{`
//     pub fn open_brace(&mut self) {
//         self.stack.push(FlowDelim::Brace);
//     }

//     /// Close a flow sequence `]`
//     pub fn close_bracket(&mut self) -> Result<(), Error> {
//         match self.stack.pop() {
//             Some(FlowDelim::Bracket) => Ok(()),
//             Some(FlowDelim::Brace) => {
//                 Err(Error::Parse("Mismatched flow delimiters: expected '}', found ']'".into()))
//             }
//             None => Err(Error::Parse("Unexpected ']': no matching '['".into())),
//         }
//     }

//     /// Close a flow mapping `}`
//     pub fn close_brace(&mut self) -> Result<(), Error> {
//         match self.stack.pop() {
//             Some(FlowDelim::Brace) => Ok(()),
//             Some(FlowDelim::Bracket) => {
//                 Err(Error::Parse("Mismatched flow delimiters: expected ']', found '}'".into()))
//             }
//             None => Err(Error::Parse("Unexpected '}': no matching '{'".into())),
//         }
//     }

//     /// Validate that all flow collections are closed
//     pub fn check_all_closed(&self) -> Result<(), Error> {
//         if let Some(delim) = self.stack.last() {
//             match delim {
//                 FlowDelim::Bracket => Err(Error::Parse("Unclosed '[': missing ']'".into())),
//                 FlowDelim::Brace => Err(Error::Parse("Unclosed '{': missing '}'".into())),
//             }
//         } else {
//             Ok(())
//         }
//     }
// }

// /// Validate trailing comma in flow collection
// ///
// /// YAML does not allow trailing commas: `[a, b,]` or `{a: 1,}` are invalid
// #[allow(dead_code)]
// pub fn validate_no_trailing_comma(
//     prev_tok: &Tok,
//     next_tok: &Tok,
// ) -> Result<(), Error> {
//     // Check if previous token was a comma and next is a closing delimiter
//     if matches!(prev_tok, Tok::Comma) {
//         match next_tok {
//             Tok::RBracket => {
//                 return Err(Error::Parse(
//                     "Trailing comma not allowed in flow sequence".into(),
//                 ))
//             }
//             Tok::RBrace => {
//                 return Err(Error::Parse(
//                     "Trailing comma not allowed in flow mapping".into(),
//                 ))
//             }
//             _ => {}
//         }
//     }
//     Ok(())
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_matched_brackets() {
//         let mut v = FlowValidator::new();
//         v.open_bracket();
//         assert!(v.in_flow());
//         assert!(v.close_bracket().is_ok());
//         assert!(!v.in_flow());
//         assert!(v.check_all_closed().is_ok());
//     }

//     #[test]
//     fn test_matched_braces() {
//         let mut v = FlowValidator::new();
//         v.open_brace();
//         assert!(v.in_flow());
//         assert!(v.close_brace().is_ok());
//         assert!(!v.in_flow());
//         assert!(v.check_all_closed().is_ok());
//     }

//     #[test]
//     fn test_unmatched_bracket() {
//         let mut v = FlowValidator::new();
//         v.open_bracket();
//         let result = v.close_brace();
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_unmatched_brace() {
//         let mut v = FlowValidator::new();
//         v.open_brace();
//         let result = v.close_bracket();
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_unclosed_bracket() {
//         let mut v = FlowValidator::new();
//         v.open_bracket();
//         let result = v.check_all_closed();
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_extra_close() {
//         let mut v = FlowValidator::new();
//         let result = v.close_bracket();
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_nested_collections() {
//         let mut v = FlowValidator::new();
//         v.open_bracket(); // [
//         v.open_brace();   // {
//         assert!(v.close_brace().is_ok());  // }
//         assert!(v.close_bracket().is_ok()); // ]
//         assert!(v.check_all_closed().is_ok());
//     }

//     #[test]
//     fn test_trailing_comma_sequence() {
//         let result = validate_no_trailing_comma(&Tok::Comma, &Tok::RBracket);
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_trailing_comma_mapping() {
//         let result = validate_no_trailing_comma(&Tok::Comma, &Tok::RBrace);
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_no_trailing_comma() {
//         let result = validate_no_trailing_comma(
//             &Tok::Str(crate::lexer::StrTok {
//                 text: "a".to_string(),
//                 quoted: false,
//             }),
//             &Tok::RBracket,
//         );
//         assert!(result.is_ok());
//     }
// }
