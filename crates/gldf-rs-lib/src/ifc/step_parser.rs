//! IFC STEP file parser
//!
//! Parses IFC files in STEP Physical File Format (ISO 10303-21)
//! to extract luminaire data for conversion to GLDF.

use anyhow::Result;
use std::collections::HashMap;

/// Parsed STEP entity with ID and data
#[derive(Debug, Clone)]
pub struct StepEntity {
    /// Entity ID (e.g., 123 from #123)
    pub id: u64,
    /// Entity type (e.g., "IFCLIGHTFIXTURETYPE")
    pub entity_type: String,
    /// Raw parameter string
    pub params: String,
    /// Parsed parameters (lazy parsing)
    pub parsed_params: Option<Vec<StepValue>>,
}

/// STEP parameter value types
#[derive(Debug, Clone, PartialEq)]
pub enum StepValue {
    /// Reference to another entity (#123)
    Ref(u64),
    /// String value ('text')
    String(String),
    /// Integer value
    Integer(i64),
    /// Real/float value
    Real(f64),
    /// Enumeration (.ENUM.)
    Enum(String),
    /// Null/undefined ($)
    Null,
    /// Derived (*)
    Derived,
    /// List of values ((...))
    List(Vec<StepValue>),
    /// Typed value like IFCLABEL('text')
    Typed(String, Box<StepValue>),
}

impl StepValue {
    /// Get as entity reference
    pub fn as_ref(&self) -> Option<u64> {
        match self {
            StepValue::Ref(id) => Some(*id),
            _ => None,
        }
    }

    /// Get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            StepValue::String(s) => Some(s),
            StepValue::Typed(_, inner) => inner.as_string(),
            _ => None,
        }
    }

    /// Get as f64
    pub fn as_real(&self) -> Option<f64> {
        match self {
            StepValue::Real(r) => Some(*r),
            StepValue::Integer(i) => Some(*i as f64),
            StepValue::Typed(_, inner) => inner.as_real(),
            _ => None,
        }
    }

    /// Get as i64
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            StepValue::Integer(i) => Some(*i),
            StepValue::Real(r) => Some(*r as i64),
            StepValue::Typed(_, inner) => inner.as_integer(),
            _ => None,
        }
    }

    /// Get as enum string
    pub fn as_enum(&self) -> Option<&str> {
        match self {
            StepValue::Enum(e) => Some(e),
            _ => None,
        }
    }

    /// Get as list
    pub fn as_list(&self) -> Option<&Vec<StepValue>> {
        match self {
            StepValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        matches!(self, StepValue::Null)
    }

    /// Get as boolean
    /// IFC booleans are represented as .T. (true) or .F. (false) enums
    /// Also handles IFCBOOLEAN(true/false)
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            StepValue::Enum(e) => match e.to_uppercase().as_str() {
                "T" | "TRUE" => Some(true),
                "F" | "FALSE" => Some(false),
                _ => None,
            },
            StepValue::Typed(type_name, inner) => {
                if type_name.to_uppercase() == "IFCBOOLEAN" {
                    inner.as_boolean()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// STEP file parser for IFC
pub struct StepParser {
    /// All entities by ID
    pub entities: HashMap<u64, StepEntity>,
    /// Schema (e.g., "IFC4")
    pub schema: String,
    /// File description
    pub description: String,
    /// File name
    pub file_name: String,
}

impl StepParser {
    /// Parse a STEP file from string content
    pub fn parse(content: &str) -> Result<Self> {
        let mut parser = Self {
            entities: HashMap::new(),
            schema: String::new(),
            description: String::new(),
            file_name: String::new(),
        };

        let mut in_data = false;
        let mut current_line = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with("/*") {
                continue;
            }

            // Track section
            if line == "DATA;" {
                in_data = true;
                continue;
            }
            if line == "ENDSEC;" {
                in_data = false;
                continue;
            }

            // Parse header
            if !in_data {
                if line.starts_with("FILE_SCHEMA") {
                    parser.schema = Self::extract_schema(line);
                } else if line.starts_with("FILE_DESCRIPTION") {
                    parser.description = Self::extract_first_string(line);
                } else if line.starts_with("FILE_NAME") {
                    parser.file_name = Self::extract_first_string(line);
                }
                continue;
            }

            // Accumulate multi-line entities
            current_line.push_str(line);
            if !line.ends_with(';') {
                current_line.push(' ');
                continue;
            }

            // Parse complete entity line
            if let Some(entity) = Self::parse_entity_line(&current_line) {
                parser.entities.insert(entity.id, entity);
            }

            current_line.clear();
        }

        Ok(parser)
    }

    /// Extract schema from FILE_SCHEMA line
    fn extract_schema(line: &str) -> String {
        // FILE_SCHEMA(('IFC4'));
        if let Some(start) = line.find("'") {
            if let Some(end) = line[start + 1..].find("'") {
                return line[start + 1..start + 1 + end].to_string();
            }
        }
        String::new()
    }

    /// Extract first string from a line
    fn extract_first_string(line: &str) -> String {
        if let Some(start) = line.find("'") {
            if let Some(end) = line[start + 1..].find("'") {
                return line[start + 1..start + 1 + end].to_string();
            }
        }
        String::new()
    }

    /// Parse a single entity line like "#123=IFCTYPE(params);"
    fn parse_entity_line(line: &str) -> Option<StepEntity> {
        let line = line.trim().trim_end_matches(';');

        // Find entity ID
        if !line.starts_with('#') {
            return None;
        }

        let eq_pos = line.find('=')?;
        let id: u64 = line[1..eq_pos].parse().ok()?;

        // Find entity type and params
        let rest = &line[eq_pos + 1..];
        let paren_pos = rest.find('(')?;
        let entity_type = rest[..paren_pos].trim().to_uppercase();

        // Extract params (everything between outer parentheses)
        let params_start = paren_pos + 1;
        let params_end = rest.rfind(')')?;
        let params = rest[params_start..params_end].to_string();

        Some(StepEntity {
            id,
            entity_type,
            params,
            parsed_params: None,
        })
    }

    /// Parse parameter string into StepValue list
    pub fn parse_params(params: &str) -> Vec<StepValue> {
        let mut result = Vec::new();
        let mut chars = params.chars().peekable();

        while chars.peek().is_some() {
            // Skip whitespace and commas
            while matches!(chars.peek(), Some(' ') | Some(',')) {
                chars.next();
            }

            if chars.peek().is_none() {
                break;
            }

            if let Some(value) = Self::parse_value(&mut chars) {
                result.push(value);
            }
        }

        result
    }

    /// Parse a single value from char iterator
    fn parse_value(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<StepValue> {
        // Skip whitespace
        while matches!(chars.peek(), Some(' ')) {
            chars.next();
        }

        match chars.peek()? {
            '#' => {
                // Entity reference
                chars.next(); // consume #
                let mut num = String::new();
                while matches!(chars.peek(), Some('0'..='9')) {
                    num.push(chars.next().unwrap());
                }
                Some(StepValue::Ref(num.parse().ok()?))
            }
            '\'' => {
                // String
                chars.next(); // consume opening '
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '\'' {
                        chars.next();
                        // Check for escaped quote ''
                        if chars.peek() == Some(&'\'') {
                            s.push('\'');
                            chars.next();
                        } else {
                            break;
                        }
                    } else {
                        s.push(c);
                        chars.next();
                    }
                }
                Some(StepValue::String(s))
            }
            '.' => {
                // Enumeration
                chars.next(); // consume opening .
                let mut e = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '.' {
                        chars.next();
                        break;
                    }
                    e.push(c);
                    chars.next();
                }
                Some(StepValue::Enum(e))
            }
            '$' => {
                chars.next();
                Some(StepValue::Null)
            }
            '*' => {
                chars.next();
                Some(StepValue::Derived)
            }
            '(' => {
                // List
                chars.next(); // consume (
                let mut list = Vec::new();
                loop {
                    // Skip whitespace and commas
                    while matches!(chars.peek(), Some(' ') | Some(',')) {
                        chars.next();
                    }
                    if chars.peek() == Some(&')') {
                        chars.next();
                        break;
                    }
                    if let Some(v) = Self::parse_value(chars) {
                        list.push(v);
                    } else {
                        break;
                    }
                }
                Some(StepValue::List(list))
            }
            c if c.is_ascii_alphabetic() => {
                // Typed value like IFCLABEL('text') or just a keyword
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Check if followed by (
                if chars.peek() == Some(&'(') {
                    chars.next(); // consume (
                    let inner = Self::parse_value(chars)?;
                    // Consume closing )
                    while matches!(chars.peek(), Some(' ')) {
                        chars.next();
                    }
                    if chars.peek() == Some(&')') {
                        chars.next();
                    }
                    Some(StepValue::Typed(name, Box::new(inner)))
                } else {
                    // Just a keyword/enum without dots
                    Some(StepValue::Enum(name))
                }
            }
            c if c.is_ascii_digit() || *c == '-' || *c == '+' => {
                // Number
                let mut num = String::new();
                let mut is_real = false;

                // Sign
                if matches!(chars.peek(), Some('-') | Some('+')) {
                    num.push(chars.next().unwrap());
                }

                // Digits before decimal
                while matches!(chars.peek(), Some('0'..='9')) {
                    num.push(chars.next().unwrap());
                }

                // Decimal point
                if chars.peek() == Some(&'.') {
                    is_real = true;
                    num.push(chars.next().unwrap());
                    while matches!(chars.peek(), Some('0'..='9')) {
                        num.push(chars.next().unwrap());
                    }
                }

                // Exponent
                if matches!(chars.peek(), Some('E') | Some('e')) {
                    is_real = true;
                    num.push(chars.next().unwrap());
                    if matches!(chars.peek(), Some('-') | Some('+')) {
                        num.push(chars.next().unwrap());
                    }
                    while matches!(chars.peek(), Some('0'..='9')) {
                        num.push(chars.next().unwrap());
                    }
                }

                if is_real {
                    Some(StepValue::Real(num.parse().ok()?))
                } else {
                    Some(StepValue::Integer(num.parse().ok()?))
                }
            }
            _ => {
                // Unknown, skip
                chars.next();
                None
            }
        }
    }

    /// Get entity by ID
    pub fn get(&self, id: u64) -> Option<&StepEntity> {
        self.entities.get(&id)
    }

    /// Get entity and parse its params
    pub fn get_parsed(&self, id: u64) -> Option<(&StepEntity, Vec<StepValue>)> {
        let entity = self.entities.get(&id)?;
        let params = Self::parse_params(&entity.params);
        Some((entity, params))
    }

    /// Find all entities of a given type
    pub fn find_by_type(&self, entity_type: &str) -> Vec<&StepEntity> {
        let upper = entity_type.to_uppercase();
        self.entities
            .values()
            .filter(|e| e.entity_type == upper)
            .collect()
    }

    /// Follow a reference chain to get the final entity
    pub fn resolve_ref(&self, value: &StepValue) -> Option<&StepEntity> {
        match value {
            StepValue::Ref(id) => self.get(*id),
            _ => None,
        }
    }
}

impl StepEntity {
    /// Get parsed parameters (parses lazily if needed)
    pub fn get_params(&self) -> Vec<StepValue> {
        StepParser::parse_params(&self.params)
    }

    /// Get parameter at index
    pub fn param(&self, index: usize) -> Option<StepValue> {
        self.get_params().get(index).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_values() {
        let params = StepParser::parse_params("'hello',123,45.67,$,*,.ENUM.");
        assert_eq!(params.len(), 6);
        assert_eq!(params[0].as_string(), Some("hello"));
        assert_eq!(params[1].as_integer(), Some(123));
        assert_eq!(params[2].as_real(), Some(45.67));
        assert!(params[3].is_null());
        assert!(matches!(params[4], StepValue::Derived));
        assert_eq!(params[5].as_enum(), Some("ENUM"));
    }

    #[test]
    fn test_parse_references() {
        let params = StepParser::parse_params("#1,#23,#456");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].as_ref(), Some(1));
        assert_eq!(params[1].as_ref(), Some(23));
        assert_eq!(params[2].as_ref(), Some(456));
    }

    #[test]
    fn test_parse_list() {
        let params = StepParser::parse_params("(1,2,3),'text',(#1,#2)");
        assert_eq!(params.len(), 3);

        let list1 = params[0].as_list().unwrap();
        assert_eq!(list1.len(), 3);
        assert_eq!(list1[0].as_integer(), Some(1));

        let list2 = params[2].as_list().unwrap();
        assert_eq!(list2.len(), 2);
        assert_eq!(list2[0].as_ref(), Some(1));
    }

    #[test]
    fn test_parse_typed() {
        let params = StepParser::parse_params("IFCLABEL('Test'),IFCREAL(123.45)");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].as_string(), Some("Test"));
        assert_eq!(params[1].as_real(), Some(123.45));
    }

    #[test]
    fn test_parse_entity_line() {
        let line = "#123=IFCLIGHTFIXTURETYPE('guid',#5,'Name','Desc',$,$,$,$,$,.POINTSOURCE.);";
        let entity = StepParser::parse_entity_line(line).unwrap();

        assert_eq!(entity.id, 123);
        assert_eq!(entity.entity_type, "IFCLIGHTFIXTURETYPE");

        let params = entity.get_params();
        assert_eq!(params[0].as_string(), Some("guid"));
        assert_eq!(params[1].as_ref(), Some(5));
        assert_eq!(params[2].as_string(), Some("Name"));
    }

    #[test]
    fn test_parse_step_file() {
        let content = r#"
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Test'),'2;1');
FILE_NAME('test.ifc','2024-01-01',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;

DATA;
#1=IFCPERSON($,$,'Author',$,$,$,$,$);
#2=IFCORGANIZATION($,'Test Corp','GLDF Export',$,$);
#3=IFCLIGHTFIXTURETYPE('guid',#1,'LED Panel','Description',$,$,$,$,$,.POINTSOURCE.);
ENDSEC;

END-ISO-10303-21;
"#;

        let parser = StepParser::parse(content).unwrap();
        assert_eq!(parser.schema, "IFC4");
        assert_eq!(parser.entities.len(), 3);

        let fixture_types = parser.find_by_type("IFCLIGHTFIXTURETYPE");
        assert_eq!(fixture_types.len(), 1);
        assert_eq!(fixture_types[0].id, 3);
    }
}
