use crate::env::Env;
use crate::printer::pr_str;
use crate::types::MalError::*;
use std::cell::RefCell;
use std::collections::hash_map;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::iter::FusedIterator;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct MalValue {
    pub mal_type: Rc<MalValueType>,
    // Possible extra fields: line, column
}

impl MalValue {
    pub fn new(mal_type: MalValueType) -> MalValue {
        MalValue {
            mal_type: Rc::new(mal_type),
        }
    }

    pub fn new_boolean(boolean: bool) -> MalValue {
        if boolean {
            MalValue::new(MalValueType::True)
        } else {
            MalValue::new(MalValueType::False)
        }
    }

    pub fn new_rust_func(func: fn(&[MalValue], &mut Env) -> MalResult, env: &Env) -> MalValue {
        MalValue::new(MalValueType::RustFunc(RustFunction {
            func,
            env: env.clone(),
        }))
    }

    pub fn new_mal_func(body: MalValue, parameters: Vec<String>, outer_env: Env) -> MalValue {
        MalValue::new(MalValueType::MalFunc(MalFunction {
            body,
            parameters,
            outer_env,
            is_macro: false,
        }))
    }

    pub fn new_mal_macro(body: MalValue, parameters: Vec<String>, outer_env: Env) -> MalValue {
        MalValue::new(MalValueType::MalFunc(MalFunction {
            body,
            parameters,
            outer_env,
            is_macro: true,
        }))
    }

    pub fn new_atom(value: MalValue) -> MalValue {
        MalValue::new(MalValueType::Atom(RefCell::new(value)))
    }

    pub fn nil() -> MalValue {
        MalValue::new(MalValueType::Nil)
    }

    pub fn is_list(&self) -> bool {
        if let MalValueType::List(_) = *self.mal_type {
            true
        } else {
            false
        }
    }

    pub fn is_function(&self) -> bool {
        match *self.mal_type {
            MalValueType::RustFunc(_) | MalValueType::MalFunc(_) => true,
            _ => false,
        }
    }

    pub fn is_atom(&self) -> bool {
        if let MalValueType::Atom(_) = *self.mal_type {
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub enum MalValueType {
    Nil,
    True,
    False,
    Number(f64),
    Symbol(String),
    Str(String),
    Keyword(String),
    List(Vec<MalValue>),
    Vector(Vec<MalValue>),
    Map(MalMap),
    RustFunc(RustFunction),
    MalFunc(MalFunction),
    Atom(RefCell<MalValue>),
}

impl PartialEq for MalValueType {
    fn eq(&self, other: &MalValueType) -> bool {
        use crate::types::MalValueType::*;

        match (self, other) {
            (Nil, Nil) => true,
            (True, True) => true,
            (False, False) => true,
            (Number(l), Number(r)) => l == r,
            (Symbol(l), Symbol(r)) => l == r,
            (Str(l), Str(r)) => l == r,
            (Keyword(l), Keyword(r)) => l == r,
            (List(l), List(r))
            | (Vector(l), Vector(r))
            | (List(l), Vector(r))
            | (Vector(l), List(r)) => l == r,
            (Map(l), Map(r)) => l == r,
            (RustFunc(l), RustFunc(r)) => l == r,
            (MalFunc(l), MalFunc(r)) => l == r,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MalMap {
    map: HashMap<MalMapKey, MalValue>,
}

#[derive(Clone, Debug)]
struct MalMapKey {
    key: String,
    mal_value: MalValue,
}

impl PartialEq for MalMapKey {
    fn eq(&self, other: &MalMapKey) -> bool {
        self.key == other.key
    }
}

impl Eq for MalMapKey {}

impl Hash for MalMapKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl MalMap {
    pub fn new() -> MalMap {
        MalMap {
            map: HashMap::new(),
        }
    }

    pub fn from_arguments(arguments: &[MalValue]) -> Result<MalMap, MalError> {
        if arguments.len() % 2 != 0 {
            return Err(MalError::Parser(
                "hash map must have an even number of arguments".to_string(),
            ));
        }

        let mut map = HashMap::with_capacity(arguments.len() % 2);

        for i in (0..arguments.len()).step_by(2) {
            let key = match *arguments[i].mal_type {
                MalValueType::Str(ref val) => Ok(format!("s{}", val)),
                MalValueType::Keyword(ref val) => Ok(format!("k{}", val)),
                _ => Err(MalError::Parser(
                    "hash map keys must be strings or keywords".to_string(),
                )),
            }?;

            map.insert(
                MalMapKey {
                    key,
                    mal_value: arguments[i].clone(),
                },
                arguments[i + 1].clone(),
            );
        }

        Ok(MalMap { map })
    }

    pub fn iter(&self) -> MalMapIter {
        MalMapIter {
            inner: self.map.iter(),
        }
    }
}

impl Default for MalMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct MalMapIter<'a> {
    inner: hash_map::Iter<'a, MalMapKey, MalValue>,
}

impl<'a> Iterator for MalMapIter<'a> {
    type Item = (&'a MalValue, &'a MalValue);

    #[inline]
    fn next(&mut self) -> Option<(&'a MalValue, &'a MalValue)> {
        let inner_next = self.inner.next();

        inner_next.map(|(key, val)| (&key.mal_value, val))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for MalMapIter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> FusedIterator for MalMapIter<'a> {}

pub struct RustFunction {
    pub func: fn(&[MalValue], &mut Env) -> MalResult,
    pub env: Env,
}

impl fmt::Debug for RustFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RustFunction")
            .field("func", &(self.func as usize))
            .field("env", &self.env)
            .finish()
    }
}

impl PartialEq for RustFunction {
    fn eq(&self, other: &RustFunction) -> bool {
        (self.func as usize == other.func as usize) && (self.env == other.env)
    }
}

#[derive(Debug, PartialEq)]
pub struct MalFunction {
    pub body: MalValue,
    pub parameters: Vec<String>,
    pub outer_env: Env,
    pub is_macro: bool,
}

#[derive(Debug, PartialEq)]
pub enum MalError {
    EmptyProgram,
    Tokenizer(String),
    Parser(String),
    UndefinedSymbol(String),
    Evaluation(String),
    RustFunction(String),
    SpecialForm(String),
    Exception(MalValue),
}

impl fmt::Display for MalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmptyProgram => write!(f, "Empty program."),
            Tokenizer(message) => write!(f, "Tokenizer error: {}", message),
            Parser(message) => write!(f, "Parser error: {}", message),
            UndefinedSymbol(symbol) => write!(f, "'{}' not found", symbol),
            Evaluation(message) => write!(f, "Error in evaluation: {}", message),
            MalError::RustFunction(message) => {
                write!(f, "Error when calling rust function: {}", message)
            }
            MalError::SpecialForm(message) => {
                write!(f, "Error when evaluating special form: {}", message)
            }
            MalError::Exception(ref val) => write!(f, "Exception: {}", pr_str(val, true)),
        }
    }
}

pub type MalResult = Result<MalValue, MalError>;

#[derive(Debug, PartialEq)]
pub struct MalToken {
    pub token_type: MalTokenType,
    // Possible extra fields: line, column
}

impl MalToken {
    pub fn new(token_type: MalTokenType) -> MalToken {
        MalToken { token_type }
    }
}

#[derive(Debug, PartialEq)]
pub enum MalTokenType {
    LParen,
    RParen,
    LCurly,
    RCurly,
    LBracket,
    RBracket,
    AtSign,
    SingleQuote,
    BackTick,
    Tilde,
    TildeAtSign,
    Nil,
    True,
    False,
    Number(f64),
    Symbol(String),
    Str(String),
    Keyword(String),
}
