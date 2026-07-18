use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{HashMap as StdHashMap, HashSet as StdHashSet},
    fmt,
    hash::{BuildHasherDefault, Hasher},
    io::{self, Write},
    rc::Rc,
};

use crate::{
    ast::*,
    diagnostics::Diagnostic,
    lexer::Lexer,
    numeric_vm::{
        self, BoolExpr as VmBoolExpr, CompareOp as VmCompareOp, FloatExpr as VmFloatExpr,
        FloatListOp, IntExpr as VmIntExpr, IntListOp, NumericCall, NumericFunction, NumericListRef,
        NumericParameter, NumericStmt, NumericValueExpr,
    },
    parser::Parser,
    token::Span,
};

struct FnvHasher(u64);

impl Default for FnvHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for FnvHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type HashMap<K, V> = StdHashMap<K, V, BuildHasherDefault<FnvHasher>>;

#[derive(Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Rc<RefCell<Vec<Value>>>, bool),
    Dictionary(Rc<RefCell<DictionaryValue>>, bool),
    Range(i64, i64),
    Function(Rc<FunctionValue>),
    Class(Rc<ClassValue>),
    Instance(Rc<RefCell<InstanceValue>>, bool),
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum ScalarKey {
    None,
    Bool(bool),
    Int(i64),
    Float(u64),
    String(String),
}

#[derive(Clone)]
struct DictionaryEntry {
    key: Value,
    value: Value,
}

pub struct DictionaryValue {
    entries: Vec<DictionaryEntry>,
    index: HashMap<ScalarKey, usize>,
    key_type: Option<TypeRef>,
    value_type: Option<TypeRef>,
}

impl DictionaryValue {
    fn empty() -> Self {
        Self {
            entries: vec![],
            index: HashMap::default(),
            key_type: None,
            value_type: None,
        }
    }

    fn position(&self, key: &ScalarKey) -> Option<usize> {
        self.index.get(key).copied()
    }

    fn insert(&mut self, scalar: ScalarKey, key: Value, value: Value) -> bool {
        if let Some(position) = self.position(&scalar) {
            self.entries[position].value = value;
            false
        } else {
            let position = self.entries.len();
            self.entries.push(DictionaryEntry { key, value });
            self.index.insert(scalar, position);
            true
        }
    }

    fn remove(&mut self, key: &ScalarKey) -> bool {
        let Some(position) = self.index.remove(key) else {
            return false;
        };
        self.entries.remove(position);
        for index in self.index.values_mut() {
            if *index > position {
                *index -= 1;
            }
        }
        true
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
}

#[derive(Clone)]
pub struct FunctionValue {
    params: Vec<Param>,
    return_type: Option<TypeRef>,
    body: Block,
    numeric: Option<Rc<NumericFunction>>,
}

#[derive(Clone)]
pub struct ClassValue {
    name: String,
    fields: Vec<FieldDefinition>,
    methods: HashMap<String, MethodDefinition>,
}

#[derive(Clone)]
struct FieldDefinition {
    name: String,
    value: Expr,
    private: bool,
}

#[derive(Clone)]
struct MethodDefinition {
    function: Rc<FunctionValue>,
    private: bool,
}

pub struct InstanceValue {
    class: Rc<ClassValue>,
    fields: HashMap<String, Value>,
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::List(..) => "List",
            Self::Dictionary(..) => "Dictionary",
            Self::Range(..) => "Range",
            Self::Function(_) => "Function",
            Self::Class(_) => "Class",
            Self::Instance(..) => "Object",
        }
    }
    fn truthy(&self) -> bool {
        match self {
            Self::None => false,
            Self::Bool(v) => *v,
            Self::Int(v) => *v != 0,
            Self::Float(v) => *v != 0.0,
            Self::String(v) => !v.is_empty(),
            Self::List(v, _) => !v.borrow().is_empty(),
            Self::Dictionary(v, _) => !v.borrow().entries.is_empty(),
            Self::Range(a, b) => a != b,
            Self::Function(_) => true,
            Self::Class(_) | Self::Instance(..) => true,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn display(
            value: &Value,
            f: &mut fmt::Formatter<'_>,
            active: &mut StdHashSet<(u8, usize)>,
        ) -> fmt::Result {
            if let Value::List(items, _) = value {
                let pointer = (0, Rc::as_ptr(items) as usize);
                if !active.insert(pointer) {
                    return write!(f, "[<cycle>]");
                }
                write!(f, "[")?;
                for (i, item) in items.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match item {
                        Value::String(text) => write!(f, "\"{text}\"")?,
                        _ => display(item, f, active)?,
                    }
                }
                active.remove(&pointer);
                return write!(f, "]");
            }
            if let Value::Dictionary(dictionary, _) = value {
                let pointer = (1, Rc::as_ptr(dictionary) as usize);
                if !active.insert(pointer) {
                    return write!(f, "{{<cycle>}}");
                }
                write!(f, "{{")?;
                for (index, entry) in dictionary.borrow().entries.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    if let Value::String(text) = &entry.key {
                        write!(f, "\"{text}\"")?;
                    } else {
                        display(&entry.key, f, active)?;
                    }
                    write!(f, ": ")?;
                    if let Value::String(text) = &entry.value {
                        write!(f, "\"{text}\"")?;
                    } else {
                        display(&entry.value, f, active)?;
                    }
                }
                active.remove(&pointer);
                return write!(f, "}}");
            }
            match value {
                Value::None => write!(f, "none"),
                Value::Bool(v) => write!(f, "{v}"),
                Value::Int(v) => write!(f, "{v}"),
                Value::Float(v) => {
                    if v.is_nan() {
                        write!(f, "NAN")
                    } else if v.is_infinite() {
                        if v.is_sign_negative() {
                            write!(f, "-INF")
                        } else {
                            write!(f, "INF")
                        }
                    } else {
                        write!(f, "{v}")
                    }
                }
                Value::String(v) => write!(f, "{v}"),
                Value::List(..) => unreachable!("lists are handled before scalar formatting"),
                Value::Dictionary(..) => {
                    unreachable!("dictionaries are handled before scalar formatting")
                }
                Value::Range(a, b) => write!(f, "{a}..{b}"),
                Value::Function(_) => write!(f, "<function>"),
                Value::Class(class) => write!(f, "<class {}>", class.name),
                Value::Instance(instance, _) => {
                    write!(f, "<{} object>", instance.borrow().class.name)
                }
            }
        }
        display(self, f, &mut StdHashSet::new())
    }
}

impl PartialEq for Value {
    fn eq(&self, o: &Self) -> bool {
        fn equal(a: &Value, b: &Value, seen: &mut StdHashSet<(u8, usize, usize)>) -> bool {
            if let (Value::List(left, _), Value::List(right, _)) = (a, b) {
                let pair = (0, Rc::as_ptr(left) as usize, Rc::as_ptr(right) as usize);
                if !seen.insert(pair) {
                    return true;
                }
                let left_values = left.borrow();
                let right_values = right.borrow();
                return left_values.len() == right_values.len()
                    && left_values
                        .iter()
                        .zip(right_values.iter())
                        .all(|(left, right)| equal(left, right, seen));
            }
            if let (Value::Dictionary(left, _), Value::Dictionary(right, _)) = (a, b) {
                let pair = (1, Rc::as_ptr(left) as usize, Rc::as_ptr(right) as usize);
                if !seen.insert(pair) {
                    return true;
                }
                let left = left.borrow();
                let right = right.borrow();
                if left.entries.len() != right.entries.len() {
                    return false;
                }
                return left.entries.iter().all(|entry| {
                    let Some(position) =
                        scalar_key(&entry.key).and_then(|key| right.position(&key))
                    else {
                        return false;
                    };
                    equal(&entry.value, &right.entries[position].value, seen)
                });
            }
            match (a, b) {
                (Value::None, Value::None) => true,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::Int(a), Value::Int(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Int(a), Value::Float(b)) => {
                    compare_int_float_exact(*a, *b) == Some(Ordering::Equal)
                }
                (Value::Float(a), Value::Int(b)) => {
                    compare_int_float_exact(*b, *a) == Some(Ordering::Equal)
                }
                (Value::String(a), Value::String(b)) => a == b,
                (Value::List(..), Value::List(..)) => {
                    unreachable!("lists are handled before scalar equality")
                }
                (Value::Dictionary(..), Value::Dictionary(..)) => {
                    unreachable!("dictionaries are handled before scalar equality")
                }
                (Value::Class(a), Value::Class(b)) => Rc::ptr_eq(a, b),
                (Value::Instance(a, _), Value::Instance(b, _)) => Rc::ptr_eq(a, b),
                _ => false,
            }
        }
        equal(self, o, &mut StdHashSet::new())
    }
}

#[derive(Clone)]
struct Binding {
    value: Value,
    ty: Option<TypeRef>,
    constant: bool,
}

#[derive(Default)]
struct Scope {
    names: HashMap<String, usize>,
    bindings: Vec<Binding>,
}

impl Scope {
    fn contains_key(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    fn get(&self, name: &str) -> Option<&Binding> {
        self.names.get(name).map(|index| &self.bindings[*index])
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut Binding> {
        let index = *self.names.get(name)?;
        Some(&mut self.bindings[index])
    }

    fn insert(&mut self, name: String, binding: Binding) {
        if let Some(index) = self.names.get(&name).copied() {
            self.bindings[index] = binding;
        } else {
            let index = self.bindings.len();
            self.bindings.push(binding);
            self.names.insert(name, index);
        }
    }

    fn values(&self) -> impl Iterator<Item = &Binding> {
        self.bindings.iter()
    }

    fn retain_only(&mut self, name: &str) {
        if self.names.len() <= 1 {
            return;
        }
        let index = self.names[name];
        if index != 0 {
            self.bindings.swap(0, index);
        }
        self.bindings.truncate(1);
        self.names.clear();
        self.names.insert(name.to_owned(), 0);
    }

    fn binding(&self, index: usize) -> &Binding {
        &self.bindings[index]
    }

    fn binding_mut(&mut self, index: usize) -> &mut Binding {
        &mut self.bindings[index]
    }
}

#[derive(Clone, Copy)]
enum CachedLocation {
    Absolute { scope: usize, slot: usize },
    Relative { depth: usize, slot: usize },
    Missing,
}

#[derive(Clone, Copy)]
struct BindingCacheEntry {
    key: usize,
    generation: u64,
    location: CachedLocation,
}

impl Default for BindingCacheEntry {
    fn default() -> Self {
        Self {
            key: 0,
            generation: 0,
            location: CachedLocation::Absolute { scope: 0, slot: 0 },
        }
    }
}

const BINDING_CACHE_SIZE: usize = 2048;
const MAX_LOOP_ITERATIONS: usize = 10_000_000;

#[derive(Clone, Copy)]
struct FastNumber {
    value: f64,
    is_float: bool,
}
enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

enum NumericFlow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

#[derive(Default)]
struct NumericFrame {
    integers: Vec<i64>,
    floats: Vec<f64>,
    bools: Vec<bool>,
    int_lists: Vec<Vec<i64>>,
    float_lists: Vec<Vec<f64>>,
}

impl NumericFrame {
    fn prepare(&mut self, function: &NumericFunction) {
        self.integers.resize(function.int_locals, 0);
        self.integers.fill(0);
        self.floats.resize(function.float_locals, 0.0);
        self.floats.fill(0.0);
        self.bools.resize(function.bool_locals, false);
        self.bools.fill(false);

        self.int_lists
            .resize_with(function.int_list_locals, Vec::new);
        self.int_lists.truncate(function.int_list_locals);
        for list in &mut self.int_lists {
            list.clear();
        }
        self.float_lists
            .resize_with(function.float_list_locals, Vec::new);
        self.float_lists.truncate(function.float_list_locals);
        for list in &mut self.float_lists {
            list.clear();
        }
    }
}

#[derive(Clone, Copy)]
enum NumericNumber {
    Int(i64),
    Float(f64),
}

pub struct Evaluator<'a> {
    source: &'a str,
    file: &'a str,
    output: bool,
    scopes: Vec<Scope>,
    binding_cache: Vec<BindingCacheEntry>,
    binding_cache_generation: u64,
    frame_bases: Vec<usize>,
    numeric_frame_pool: Vec<NumericFrame>,
    call_depth: usize,
    class_stack: Vec<String>,
}

impl<'a> Evaluator<'a> {
    pub fn new(source: &'a str, file: &'a str, output: bool) -> Self {
        let mut global = Scope::default();
        for (name, value) in [
            ("PI", Value::Float(std::f64::consts::PI)),
            ("TAU", Value::Float(std::f64::consts::TAU)),
            ("E", Value::Float(std::f64::consts::E)),
            ("PHI", Value::Float(1.618_033_988_749_895)),
            ("INF", Value::Float(f64::INFINITY)),
            ("NAN", Value::Float(f64::NAN)),
        ] {
            global.insert(
                name.into(),
                Binding {
                    value,
                    ty: Some(TypeRef::Float),
                    constant: true,
                },
            );
        }
        Self {
            source,
            file,
            output,
            scopes: vec![global, Scope::default()],
            binding_cache: vec![BindingCacheEntry::default(); BINDING_CACHE_SIZE],
            binding_cache_generation: 1,
            frame_bases: vec![2],
            numeric_frame_pool: vec![],
            call_depth: 0,
            class_stack: vec![],
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), Diagnostic> {
        self.register_functions(program)?;
        for s in &program.statements {
            if !matches!(s, Stmt::Function { .. } | Stmt::Class { .. }) {
                self.exec(s)?;
            }
        }
        if self.get("main").is_some() {
            self.call_named(
                "main",
                vec![],
                Span {
                    start: 0,
                    length: 1,
                    line: 1,
                    column: 1,
                },
            )?;
        }
        Ok(())
    }
    pub fn check(&mut self, program: &Program) -> Result<(), Diagnostic> {
        self.register_functions(program)?;
        for s in &program.statements {
            if !matches!(s, Stmt::Function { .. } | Stmt::Class { .. }) {
                self.check_stmt(s)?;
            }
        }
        let funcs: Vec<Rc<FunctionValue>> = self
            .scopes
            .iter()
            .flat_map(|scope| scope.values())
            .filter_map(|b| {
                if let Value::Function(f) = &b.value {
                    Some(f.clone())
                } else {
                    None
                }
            })
            .collect();
        for f in funcs {
            self.push();
            for p in &f.params {
                let v = default_value(p.ty.as_ref());
                self.define(&p.name, v, p.ty.clone(), false, p.span)?;
            }
            for s in &f.body.statements {
                self.check_stmt(s)?;
            }
            self.pop();
        }
        Ok(())
    }

    fn register_functions(&mut self, p: &Program) -> Result<(), Diagnostic> {
        let numeric_environment = numeric_vm::collect_environment(p);
        for s in &p.statements {
            if let Stmt::Function {
                name,
                params,
                return_type,
                body,
                span,
            } = s
            {
                let numeric =
                    numeric_vm::compile_function(params, body, &numeric_environment).map(Rc::new);
                self.define(
                    name,
                    Value::Function(Rc::new(FunctionValue {
                        params: params.clone(),
                        return_type: return_type.clone(),
                        body: body.clone(),
                        numeric,
                    })),
                    None,
                    true,
                    *span,
                )?;
            } else if let Stmt::Class {
                name,
                members,
                span,
            } = s
            {
                let mut fields = vec![];
                let mut methods = HashMap::default();
                let mut names = std::collections::HashSet::new();
                for member in members {
                    let member_name = match member {
                        ClassMember::Field { name, .. } | ClassMember::Method { name, .. } => name,
                    };
                    if !names.insert(member_name.clone()) {
                        return Err(self.error(
                            "Class Error",
                            format!("duplicate member `{member_name}` in class `{name}`"),
                            *span,
                            "A class member name must be unique.",
                            "Rename or remove the duplicate member.",
                        ));
                    }
                    match member {
                        ClassMember::Field {
                            name,
                            value,
                            private,
                            ..
                        } => fields.push(FieldDefinition {
                            name: name.clone(),
                            value: value.clone(),
                            private: *private,
                        }),
                        ClassMember::Method {
                            name,
                            params,
                            return_type,
                            body,
                            private,
                            ..
                        } => {
                            methods.insert(
                                name.clone(),
                                MethodDefinition {
                                    function: Rc::new(FunctionValue {
                                        params: params.clone(),
                                        return_type: return_type.clone(),
                                        body: body.clone(),
                                        numeric: None,
                                    }),
                                    private: *private,
                                },
                            );
                        }
                    }
                }
                self.define(
                    name,
                    Value::Class(Rc::new(ClassValue {
                        name: name.clone(),
                        fields,
                        methods,
                    })),
                    None,
                    true,
                    *span,
                )?;
            }
        }
        Ok(())
    }
    fn check_stmt(&mut self, s: &Stmt) -> Result<(), Diagnostic> {
        match s {
            Stmt::If {
                condition,
                then_block,
                else_branch,
                ..
            } => {
                self.eval(condition)?;
                self.push();
                for x in &then_block.statements {
                    self.check_stmt(x)?
                }
                self.pop();
                if let Some(e) = else_branch {
                    self.check_stmt(e)?
                }
                Ok(())
            }
            Stmt::Loop { kind, body, .. } => {
                match kind {
                    LoopKind::Forever => {}
                    LoopKind::While(e) => {
                        self.eval(e)?;
                    }
                    LoopKind::For {
                        name,
                        iterable,
                        step,
                    } => {
                        let iterable_value = self.eval(iterable)?;
                        if let Some(s) = step {
                            self.eval(s)?;
                        }
                        self.push();
                        let item = match iterable_value {
                            Value::List(values, _) => {
                                values.borrow().first().cloned().unwrap_or(Value::None)
                            }
                            Value::Dictionary(dictionary, _) => dictionary
                                .borrow()
                                .entries
                                .first()
                                .map(|entry| entry.key.clone())
                                .unwrap_or(Value::None),
                            Value::String(_) => Value::String(String::new()),
                            Value::Range(_, _) => Value::Int(0),
                            _ => Value::None,
                        };
                        self.define(name, item, None, false, s.span())?;
                        for x in &body.statements {
                            self.check_stmt(x)?
                        }
                        self.pop();
                        return Ok(());
                    }
                }
                self.push();
                for x in &body.statements {
                    self.check_stmt(x)?
                }
                self.pop();
                Ok(())
            }
            Stmt::Return(v, _) => {
                if let Some(v) = v {
                    self.eval(v)?;
                }
                Ok(())
            }
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Class { .. } => Ok(()),
            _ => self.exec(s).map(|_| ()),
        }
    }

    fn exec(&mut self, s: &Stmt) -> Result<Flow, Diagnostic> {
        match s {
            Stmt::Assign {
                target,
                ty,
                value,
                constant,
                op,
                span,
            } => {
                if !constant && ty.is_none() {
                    if let Some(result) = self.try_int_assignment(target, value, *op, *span) {
                        result?;
                        return Ok(Flow::Normal);
                    }
                    if let Some(result) = self.try_float_assignment(target, value, *op, *span) {
                        result?;
                        return Ok(Flow::Normal);
                    }
                }
                let mut v = self.eval(value)?;
                if *constant {
                    v = self.freeze_value(v, *span, &mut HashMap::default())?;
                }
                self.assign_target(target, ty.clone(), v, *constant, *op, *span)?;
                Ok(Flow::Normal)
            }
            Stmt::Function { .. } | Stmt::Class { .. } => Ok(Flow::Normal),
            Stmt::Expr(e) => {
                self.eval(e)?;
                Ok(Flow::Normal)
            }
            Stmt::Return(v, _) => Ok(Flow::Return(if let Some(e) = v {
                self.eval(e)?
            } else {
                Value::None
            })),
            Stmt::Break(_) => Ok(Flow::Break),
            Stmt::Continue(_) => Ok(Flow::Continue),
            Stmt::If {
                condition,
                then_block,
                else_branch,
                ..
            } => {
                if self.eval(condition)?.truthy() {
                    self.exec_block(then_block)
                } else if let Some(other) = else_branch {
                    self.exec(other)
                } else {
                    Ok(Flow::Normal)
                }
            }
            Stmt::Loop { kind, body, span } => self.exec_loop(kind, body, *span),
        }
    }
    fn exec_block(&mut self, b: &Block) -> Result<Flow, Diagnostic> {
        self.push();
        for s in &b.statements {
            let f = self.exec(s)?;
            if !matches!(f, Flow::Normal) {
                self.pop();
                return Ok(f);
            }
        }
        self.pop();
        Ok(Flow::Normal)
    }
    fn exec_loop(&mut self, k: &LoopKind, b: &Block, span: Span) -> Result<Flow, Diagnostic> {
        match k {
            LoopKind::Forever => {
                let mut guard = 0usize;
                loop {
                    if guard >= MAX_LOOP_ITERATIONS {
                        return Err(self.error(
                            "Runtime Error",
                            "loop iteration limit exceeded",
                            span,
                            "The loop ran more than ten million times.",
                            "Add a break condition or split the work into batches.",
                        ));
                    }
                    guard += 1;
                    match self.exec_block(b)? {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
            }
            LoopKind::While(c) => {
                let mut guard = 0usize;
                while self.eval(c)?.truthy() {
                    match self.exec_block(b)? {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                    guard += 1;
                    if guard >= MAX_LOOP_ITERATIONS {
                        return Err(self.error(
                            "Runtime Error",
                            "loop iteration limit exceeded",
                            span,
                            "The loop ran more than ten million times.",
                            "Check that the loop can finish.",
                        ));
                    }
                }
            }
            LoopKind::For {
                name,
                iterable,
                step,
            } => {
                let iterable = self.eval(iterable)?;
                self.push();
                self.define(name, Value::None, None, false, span)?;
                let result = match iterable {
                    Value::Range(a, z) => {
                        let step = if let Some(e) = step {
                            as_int(self.eval(e)?, e.span(), self)?
                        } else if a <= z {
                            1
                        } else {
                            -1
                        };
                        if step == 0 {
                            return Err(self.error(
                                "Value Error",
                                "range step cannot be zero",
                                span,
                                "A zero step never advances.",
                                "Use a positive or negative integer step.",
                            ));
                        }
                        self.exec_range_values(name, a, z, step, b, span)
                    }
                    Value::List(values, _) => {
                        let values = values.borrow().clone();
                        self.exec_for_values(name, values, b, span)
                    }
                    Value::Dictionary(dictionary, _) => {
                        let keys = dictionary
                            .borrow()
                            .entries
                            .iter()
                            .map(|entry| entry.key.clone())
                            .collect::<Vec<_>>();
                        self.exec_for_values(name, keys, b, span)
                    }
                    Value::String(value) => self.exec_for_values(
                        name,
                        value
                            .chars()
                            .map(|character| Value::String(character.to_string())),
                        b,
                        span,
                    ),
                    value => Err(self.type_error(span, "an iterable", &value)),
                };
                self.pop();
                if let Flow::Return(value) = result? {
                    return Ok(Flow::Return(value));
                }
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_range_values(
        &mut self,
        name: &str,
        start: i64,
        end: i64,
        step: i64,
        body: &Block,
        span: Span,
    ) -> Result<Flow, Diagnostic> {
        let mut value = start;
        let mut iterations = 0usize;
        while if step > 0 { value < end } else { value > end } {
            if iterations >= MAX_LOOP_ITERATIONS {
                return Err(self.error(
                    "Runtime Error",
                    "loop iteration limit exceeded",
                    span,
                    "The loop ran more than ten million times.",
                    "Use a smaller range or split the work into batches.",
                ));
            }
            iterations += 1;
            match self.exec_for_iteration(name, Value::Int(value), body, span)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => return Ok(Flow::Normal),
                Flow::Return(value) => return Ok(Flow::Return(value)),
            }
            value = value.checked_add(step).ok_or_else(|| {
                self.error(
                    "Value Error",
                    "range overflow",
                    span,
                    "The range exceeded Int limits.",
                    "Use a smaller range.",
                )
            })?;
        }
        Ok(Flow::Normal)
    }

    fn exec_for_values(
        &mut self,
        name: &str,
        values: impl IntoIterator<Item = Value>,
        body: &Block,
        span: Span,
    ) -> Result<Flow, Diagnostic> {
        for (iterations, value) in values.into_iter().enumerate() {
            if iterations >= MAX_LOOP_ITERATIONS {
                return Err(self.error(
                    "Runtime Error",
                    "loop iteration limit exceeded",
                    span,
                    "The loop ran more than ten million times.",
                    "Use a smaller iterable or split the work into batches.",
                ));
            }
            match self.exec_for_iteration(name, value, body, span)? {
                Flow::Normal | Flow::Continue => {}
                Flow::Break => return Ok(Flow::Normal),
                Flow::Return(value) => return Ok(Flow::Return(value)),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_for_iteration(
        &mut self,
        name: &str,
        value: Value,
        body: &Block,
        span: Span,
    ) -> Result<Flow, Diagnostic> {
        let loop_scope = self.scopes.len() - 1;
        if self.scopes[loop_scope].names.len() > 1 {
            self.invalidate_binding_cache();
        }
        self.scopes[loop_scope].retain_only(name);
        self.scopes[loop_scope].get_mut(name).unwrap().value = value;
        for statement in &body.statements {
            let flow = self.exec(statement)?;
            if !matches!(flow, Flow::Normal) {
                return Ok(flow);
            }
        }
        let _ = span;
        Ok(Flow::Normal)
    }

    fn assign_target(
        &mut self,
        t: &AssignTarget,
        ty: Option<TypeRef>,
        value: Value,
        constant: bool,
        op: AssignOp,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match t {
            AssignTarget::Name(name, name_span) => {
                if matches!(op, AssignOp::Set) {
                    let declaration = ty.is_some() || constant;
                    let local_exists = self
                        .scopes
                        .last()
                        .is_some_and(|scope| scope.contains_key(name));
                    if declaration {
                        if local_exists {
                            return Err(self.error("Name Error",format!("variable `{name}` is already defined"),*name_span,"A variable cannot be declared twice in the same visible scope.","Choose another name or reassign without `const` or a type annotation."));
                        }
                        self.define(name, value, ty, constant, *name_span)
                    } else if let Some((scope, slot)) = self.binding_location_cached(name) {
                        self.reassign_at(scope, slot, name, value, *name_span)
                    } else {
                        self.define(name, value, ty, constant, *name_span)
                    }
                } else {
                    let (scope, slot) = self
                        .binding_location_cached(name)
                        .ok_or_else(|| self.unknown(name, *name_span))?;
                    if self.try_compound_numeric(scope, slot, name, op, &value, *name_span)? {
                        return Ok(());
                    }
                    let old = self.scopes[scope].binding(slot).value.clone();
                    let v = self.binary(
                        old,
                        match op {
                            AssignOp::Add => BinaryOp::Add,
                            AssignOp::Subtract => BinaryOp::Subtract,
                            AssignOp::Multiply => BinaryOp::Multiply,
                            AssignOp::Divide => BinaryOp::Divide,
                            AssignOp::Set => unreachable!(),
                        },
                        value,
                        span,
                    )?;
                    self.reassign_at(scope, slot, name, v, *name_span)
                }
            }
            AssignTarget::Destructure(names, target_span) => {
                let Value::List(items, _) = value else {
                    return Err(self.type_error(*target_span, "a List for destructuring", &value));
                };
                let values = items.borrow();
                if values.len() != names.len() {
                    return Err(self.error(
                        "Value Error",
                        "destructuring length does not match",
                        *target_span,
                        format!(
                            "There are {} names but {} values.",
                            names.len(),
                            values.len()
                        ),
                        "Use the same number of names and values.",
                    ));
                }
                for (name, v) in names.iter().zip(values.iter()) {
                    if let Some(scope) = self.binding_scope(name) {
                        self.reassign_in_scope(scope, name, v.clone(), *target_span)?
                    } else {
                        self.define(name, v.clone(), None, constant, *target_span)?
                    }
                }
                Ok(())
            }
            AssignTarget::Index(object, index, target_span) => {
                let expected_list_item = if let Expr::Name(name, _) = object.as_ref() {
                    self.get(name).and_then(|binding| match binding.ty {
                        Some(TypeRef::List(Some(item))) => Some(*item),
                        _ => None,
                    })
                } else {
                    None
                };
                let obj = self.eval(object)?;
                let key = self.eval(index)?;
                match obj {
                    Value::List(items, frozen) => {
                        if frozen {
                            return Err(self.const_error(*target_span));
                        }
                        if let Some(expected) = &expected_list_item {
                            self.ensure_type(&value, expected, *target_span)?;
                        }
                        if list_would_cycle(&items, &value) {
                            return Err(self.error(
                                "Value Error",
                                "cannot create a cyclic List",
                                *target_span,
                                "Cyclic Lists make copying and comparison unsafe.",
                                "Store a scalar value or a separate copy instead.",
                            ));
                        }
                        let idx = as_int(key, index.span(), self)?;
                        let mut values = items.borrow_mut();
                        let i = normalize_index(idx, values.len())
                            .ok_or_else(|| self.index_error(*target_span, idx, values.len()))?;
                        let value = if matches!(op, AssignOp::Set) {
                            value
                        } else {
                            self.compound_value(values[i].clone(), op, value, span)?
                        };
                        values[i] = value;
                        Ok(())
                    }
                    Value::Dictionary(dictionary, frozen) => {
                        if frozen {
                            return Err(self.const_error(*target_span));
                        }
                        let scalar = dictionary_key(&key, index.span(), self)?;
                        let (key_type, value_type, position) = {
                            let dictionary = dictionary.borrow();
                            (
                                dictionary.key_type.clone(),
                                dictionary.value_type.clone(),
                                dictionary.position(&scalar),
                            )
                        };
                        if let Some(expected) = &key_type {
                            self.ensure_type(&key, expected, index.span())?;
                        }
                        if let Some(expected) = &value_type {
                            self.ensure_type(&value, expected, *target_span)?;
                        }
                        let value = if matches!(op, AssignOp::Set) {
                            value
                        } else {
                            let position =
                                position.ok_or_else(|| self.key_error(*target_span, &key))?;
                            let old = dictionary.borrow().entries[position].value.clone();
                            self.compound_value(old, op, value, span)?
                        };
                        dictionary.borrow_mut().insert(scalar, key, value);
                        Ok(())
                    }
                    value => Err(self.type_error(*target_span, "a List or Dictionary", &value)),
                }
            }
            AssignTarget::Member(object, name, target_span) => {
                let object = self.eval(object)?;
                let Value::Instance(instance, frozen) = object else {
                    return Err(self.type_error(*target_span, "an Object", &object));
                };
                if frozen {
                    return Err(self.const_error(*target_span));
                }
                let class = instance.borrow().class.clone();
                if class
                    .fields
                    .iter()
                    .any(|field| field.name == *name && field.private)
                    && !self.can_access(&class.name)
                {
                    return Err(self.private_error(&class.name, name, *target_span));
                }
                let old = instance.borrow().fields.get(name).cloned();
                let value = if matches!(op, AssignOp::Set) {
                    value
                } else {
                    let old = old.ok_or_else(|| {
                        self.error(
                            "Name Error",
                            format!("object has no field `{name}`"),
                            *target_span,
                            "Compound assignment requires an existing field.",
                            "Assign the field with `=` first.",
                        )
                    })?;
                    self.binary(
                        old,
                        match op {
                            AssignOp::Add => BinaryOp::Add,
                            AssignOp::Subtract => BinaryOp::Subtract,
                            AssignOp::Multiply => BinaryOp::Multiply,
                            AssignOp::Divide => BinaryOp::Divide,
                            AssignOp::Set => unreachable!(),
                        },
                        value,
                        span,
                    )?
                };
                if value_reaches_instance(&instance, &value) {
                    return Err(self.error(
                        "Value Error",
                        "cannot create a cyclic Object",
                        *target_span,
                        "Reference-counted Objects cannot contain a path back to themselves.",
                        "Store a separate copy or keep the relationship one-way.",
                    ));
                }
                instance.borrow_mut().fields.insert(name.clone(), value);
                Ok(())
            }
        }
    }

    fn try_int_assignment(
        &mut self,
        target: &AssignTarget,
        expression: &Expr,
        operation: AssignOp,
        span: Span,
    ) -> Option<Result<(), Diagnostic>> {
        let AssignTarget::Name(name, name_span) = target else {
            return None;
        };
        let right = match self.eval_int(expression)? {
            Ok(value) => value,
            Err(error) => return Some(Err(error)),
        };
        let location = self.binding_location_cached(name);
        if matches!(operation, AssignOp::Set) {
            if let Some((scope, slot)) = location {
                let binding = self.scopes[scope].binding(slot);
                if binding.constant
                    || !matches!(binding.ty, None | Some(TypeRef::Int | TypeRef::Number))
                {
                    return None;
                }
                self.scopes[scope].binding_mut(slot).value = Value::Int(right);
                return Some(Ok(()));
            }
            return Some(self.define(name, Value::Int(right), None, false, *name_span));
        }

        let (scope, slot) = location?;
        let binding = self.scopes[scope].binding(slot);
        let Value::Int(left) = binding.value else {
            return None;
        };
        if binding.constant {
            return None;
        }
        let value = match operation {
            AssignOp::Add => left.checked_add(right),
            AssignOp::Subtract => left.checked_sub(right),
            AssignOp::Multiply => left.checked_mul(right),
            AssignOp::Divide | AssignOp::Set => return None,
        };
        let value = match value {
            Some(value) => value,
            None => return Some(Err(integer_overflow(span, self.file, self.source))),
        };
        self.scopes[scope].binding_mut(slot).value = Value::Int(value);
        Some(Ok(()))
    }

    fn eval_int(&mut self, expression: &Expr) -> Option<Result<i64, Diagnostic>> {
        match expression {
            Expr::Int(value, _) => Some(Ok(*value)),
            Expr::Name(name, _) => {
                let (scope, slot) = self.binding_location_cached(name)?;
                match self.scopes[scope].binding(slot).value {
                    Value::Int(value) => Some(Ok(value)),
                    _ => None,
                }
            }
            Expr::Unary {
                op: UnaryOp::Positive,
                value,
                ..
            } => self.eval_int(value),
            Expr::Unary {
                op: UnaryOp::Negate,
                value,
                span,
            } => {
                let value = match self.eval_int(value)? {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                Some(
                    value
                        .checked_neg()
                        .ok_or_else(|| integer_overflow(*span, self.file, self.source)),
                )
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } if matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::IntegerDivide
                    | BinaryOp::Remainder
            ) =>
            {
                let left = match self.eval_int(left)? {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                let right = match self.eval_int(right)? {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                let value = match op {
                    BinaryOp::Add => left.checked_add(right),
                    BinaryOp::Subtract => left.checked_sub(right),
                    BinaryOp::Multiply => left.checked_mul(right),
                    BinaryOp::IntegerDivide => {
                        if right == 0 {
                            return Some(Err(self.zero(*span)));
                        }
                        left.checked_div(right)
                    }
                    BinaryOp::Remainder => {
                        if right == 0 {
                            return Some(Err(self.zero(*span)));
                        }
                        left.checked_rem(right)
                    }
                    _ => unreachable!(),
                };
                Some(value.ok_or_else(|| integer_overflow(*span, self.file, self.source)))
            }
            _ => None,
        }
    }

    fn try_float_assignment(
        &mut self,
        target: &AssignTarget,
        expression: &Expr,
        operation: AssignOp,
        span: Span,
    ) -> Option<Result<(), Diagnostic>> {
        let AssignTarget::Name(name, name_span) = target else {
            return None;
        };
        let right = match self.eval_float(expression)? {
            Ok(value) => value,
            Err(error) => return Some(Err(error)),
        };
        // Keep exact runtime typing: a pure Int expression remains Int rather than
        // being accepted as an implicit Float conversion by this specialized path.
        if !right.is_float {
            return None;
        }
        let right = right.value;
        let location = self.binding_location_cached(name);
        if matches!(operation, AssignOp::Set) {
            if let Some((scope, slot)) = location {
                let binding = self.scopes[scope].binding(slot);
                if binding.constant
                    || !matches!(binding.ty, None | Some(TypeRef::Float | TypeRef::Number))
                {
                    return None;
                }
                self.scopes[scope].binding_mut(slot).value = Value::Float(right);
                return Some(Ok(()));
            }
            return Some(self.define(name, Value::Float(right), None, false, *name_span));
        }

        let (scope, slot) = location?;
        let binding = self.scopes[scope].binding(slot);
        if binding.constant {
            return None;
        }
        let left = match binding.value {
            Value::Float(value) => value,
            Value::Int(value) => value as f64,
            _ => return None,
        };
        let value = match operation {
            AssignOp::Add => left + right,
            AssignOp::Subtract => left - right,
            AssignOp::Multiply => left * right,
            AssignOp::Divide => {
                if right == 0.0 {
                    return Some(Err(self.zero(span)));
                }
                left / right
            }
            AssignOp::Set => unreachable!(),
        };
        self.scopes[scope].binding_mut(slot).value = Value::Float(value);
        Some(Ok(()))
    }

    fn eval_float(&mut self, expression: &Expr) -> Option<Result<FastNumber, Diagnostic>> {
        match expression {
            Expr::Int(value, _) => Some(Ok(FastNumber {
                value: *value as f64,
                is_float: false,
            })),
            Expr::Float(value, _) => Some(Ok(FastNumber {
                value: *value,
                is_float: true,
            })),
            Expr::Name(name, _) => {
                let (scope, slot) = self.binding_location_cached(name)?;
                match self.scopes[scope].binding(slot).value {
                    Value::Int(value) => Some(Ok(FastNumber {
                        value: value as f64,
                        is_float: false,
                    })),
                    Value::Float(value) => Some(Ok(FastNumber {
                        value,
                        is_float: true,
                    })),
                    _ => None,
                }
            }
            Expr::Unary {
                op: UnaryOp::Positive,
                value,
                ..
            } => self.eval_float(value),
            Expr::Unary {
                op: UnaryOp::Negate,
                value,
                ..
            } => Some(self.eval_float(value)?.map(|value| FastNumber {
                value: -value.value,
                is_float: value.is_float,
            })),
            Expr::Binary {
                left,
                op,
                right,
                span,
            } if matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::IntegerDivide
                    | BinaryOp::Remainder
            ) =>
            {
                let left = match self.eval_float(left)? {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                let right = match self.eval_float(right)? {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                if right.value == 0.0
                    && matches!(
                        op,
                        BinaryOp::Divide | BinaryOp::IntegerDivide | BinaryOp::Remainder
                    )
                {
                    return Some(Err(self.zero(*span)));
                }
                let is_float = left.is_float || right.is_float || matches!(op, BinaryOp::Divide);
                let value = match op {
                    BinaryOp::Add => left.value + right.value,
                    BinaryOp::Subtract => left.value - right.value,
                    BinaryOp::Multiply => left.value * right.value,
                    BinaryOp::Divide => left.value / right.value,
                    BinaryOp::IntegerDivide => (left.value / right.value).floor(),
                    BinaryOp::Remainder => left.value % right.value,
                    _ => unreachable!(),
                };
                Some(Ok(FastNumber { value, is_float }))
            }
            Expr::Call { callee, args, span } if args.len() == 1 => {
                let Expr::Name(name, _) = callee.as_ref() else {
                    return None;
                };
                if self.binding_location_cached(name).is_some() {
                    return None;
                }
                let operation = unary_math_operation(name)?;
                let input = match self.eval_float(&args[0])? {
                    Ok(value) => value,
                    Err(error) => return Some(Err(error)),
                };
                let output = operation(input.value);
                if output.is_nan() {
                    Some(Err(self.error(
                        "Math Error",
                        format!("{name} produced an invalid result"),
                        *span,
                        "The input is outside the real-number domain.",
                        "Use a value in the function's valid domain.",
                    )))
                } else {
                    Some(Ok(FastNumber {
                        value: output,
                        is_float: true,
                    }))
                }
            }
            _ => None,
        }
    }

    fn eval(&mut self, e: &Expr) -> Result<Value, Diagnostic> {
        match e {
            Expr::None(_) => Ok(Value::None),
            Expr::Bool(v, _) => Ok(Value::Bool(*v)),
            Expr::Int(v, _) => Ok(Value::Int(*v)),
            Expr::Float(v, _) => Ok(Value::Float(*v)),
            Expr::String(v, s) => self.interpolate(v, *s),
            Expr::List(items, _) => Ok(Value::List(
                Rc::new(RefCell::new(
                    items
                        .iter()
                        .map(|x| self.eval(x))
                        .collect::<Result<Vec<_>, _>>()?,
                )),
                false,
            )),
            Expr::Dictionary(entries, span) => {
                let mut dictionary = DictionaryValue::empty();
                for (key_expression, value_expression) in entries {
                    let key = self.eval(key_expression)?;
                    let scalar = dictionary_key(&key, key_expression.span(), self)?;
                    if dictionary.position(&scalar).is_some() {
                        return Err(self.error(
                            "Key Error",
                            format!("duplicate Dictionary key {}", dictionary_key_display(&key)),
                            key_expression.span(),
                            "A Dictionary literal may define each key only once.",
                            "Remove the duplicate entry or use a different key.",
                        ));
                    }
                    let value = self.eval(value_expression)?;
                    dictionary.insert(scalar, key, value);
                }
                let _ = span;
                Ok(Value::Dictionary(Rc::new(RefCell::new(dictionary)), false))
            }
            Expr::Name(n, s) => self.get_cached(n).ok_or_else(|| self.unknown(n, *s)),
            Expr::Unary { op, value, span } => {
                let v = self.eval(value)?;
                match op {
                    UnaryOp::Not => Ok(Value::Bool(!v.truthy())),
                    UnaryOp::Positive => {
                        if matches!(v, Value::Int(_) | Value::Float(_)) {
                            Ok(v)
                        } else {
                            Err(self.type_error(*span, "a Number", &v))
                        }
                    }
                    UnaryOp::Negate => match v {
                        Value::Int(x) => x.checked_neg().map(Value::Int).ok_or_else(|| {
                            self.error(
                                "Value Error",
                                "integer overflow",
                                *span,
                                "This result is outside Int limits.",
                                "Use a Float or smaller value.",
                            )
                        }),
                        Value::Float(x) => Ok(Value::Float(-x)),
                        v => Err(self.type_error(*span, "a Number", &v)),
                    },
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                if matches!(op, BinaryOp::And) {
                    let l = self.eval(left)?;
                    return if !l.truthy() {
                        Ok(Value::Bool(false))
                    } else {
                        Ok(Value::Bool(self.eval(right)?.truthy()))
                    };
                }
                if matches!(op, BinaryOp::Or) {
                    let l = self.eval(left)?;
                    return if l.truthy() {
                        Ok(Value::Bool(true))
                    } else {
                        Ok(Value::Bool(self.eval(right)?.truthy()))
                    };
                }
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                self.binary(l, *op, r, *span)
            }
            Expr::Range { start, end, span } => {
                let a = as_int(self.eval(start)?, start.span(), self)?;
                let b = as_int(self.eval(end)?, end.span(), self)?;
                let _ = span;
                Ok(Value::Range(a, b))
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj = self.eval(object)?;
                let key = self.eval(index)?;
                match obj {
                    Value::List(v, _) => {
                        let idx = as_int(key, index.span(), self)?;
                        let values = v.borrow();
                        let i = normalize_index(idx, values.len())
                            .ok_or_else(|| self.index_error(*span, idx, values.len()))?;
                        Ok(values[i].clone())
                    }
                    Value::String(v) => {
                        let idx = as_int(key, index.span(), self)?;
                        if idx >= 0 {
                            if let Ok(index) = usize::try_from(idx) {
                                if let Some(character) = v.chars().nth(index) {
                                    return Ok(Value::String(character.to_string()));
                                }
                            }
                        }
                        let len = v.chars().count();
                        let i = normalize_index(idx, len)
                            .ok_or_else(|| self.index_error(*span, idx, len))?;
                        Ok(Value::String(v.chars().nth(i).unwrap().to_string()))
                    }
                    Value::Dictionary(dictionary, _) => {
                        let scalar = dictionary_key(&key, index.span(), self)?;
                        let dictionary = dictionary.borrow();
                        let position = dictionary
                            .position(&scalar)
                            .ok_or_else(|| self.key_error(*span, &key))?;
                        Ok(dictionary.entries[position].value.clone())
                    }
                    v => Err(self.type_error(*span, "a List, String, or Dictionary", &v)),
                }
            }
            Expr::Slice {
                object,
                start,
                end,
                span,
            } => {
                let obj = self.eval(object)?;
                let a = if let Some(x) = start {
                    Some(as_int(self.eval(x)?, x.span(), self)?)
                } else {
                    None
                };
                let z = if let Some(x) = end {
                    Some(as_int(self.eval(x)?, x.span(), self)?)
                } else {
                    None
                };
                self.slice(obj, a, z, *span)
            }
            Expr::Call { callee, args, span } => {
                if let Expr::Name(name, _) = callee.as_ref() {
                    if self.get(name).is_none() {
                        if let Some(result) = self.call_builtin_expr(name, args, *span) {
                            return result;
                        }
                        let values = args
                            .iter()
                            .map(|x| self.eval(x))
                            .collect::<Result<Vec<_>, _>>()?;
                        return self.call_builtin(name, values, *span);
                    }
                    let values = args
                        .iter()
                        .map(|x| self.eval(x))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.call_named(name, values, *span)
                } else {
                    let values = args
                        .iter()
                        .map(|x| self.eval(x))
                        .collect::<Result<Vec<_>, _>>()?;
                    let f = self.eval(callee)?;
                    self.call_value(f, values, *span)
                }
            }
            Expr::MemberCall {
                object,
                name,
                args,
                span,
            } => {
                let cached_object = if let Expr::Name(object_name, _) = object.as_ref() {
                    self.get_cached_binding(object_name)
                } else {
                    None
                };
                let expected_item = cached_object.clone().and_then(|binding| match binding.ty {
                    Some(TypeRef::List(Some(item))) => Some(*item),
                    _ => None,
                });
                let obj = if let Some(binding) = cached_object {
                    binding.value
                } else {
                    self.eval(object)?
                };
                if let Value::List(items, frozen) = &obj {
                    if name == "add" && args.len() == 1 {
                        if *frozen {
                            return Err(self.const_error(*span));
                        }
                        let value = self.eval(&args[0])?;
                        if list_would_cycle(items, &value) {
                            return Err(self.error(
                                "Value Error",
                                "cannot create a cyclic List",
                                *span,
                                "Cyclic Lists make copying and comparison unsafe.",
                                "Store a scalar value or a separate copy instead.",
                            ));
                        }
                        if let Some(expected) = &expected_item {
                            self.ensure_type(&value, expected, *span)?;
                        }
                        items.borrow_mut().push(value);
                        return Ok(Value::None);
                    }
                }
                let values = args
                    .iter()
                    .map(|x| self.eval(x))
                    .collect::<Result<Vec<_>, _>>()?;
                self.member(obj, name, values, expected_item.as_ref(), *span)
            }
            Expr::Member { object, name, span } => {
                let object = self.eval(object)?;
                let Value::Instance(instance, _) = object else {
                    return Err(self.type_error(*span, "an Object", &object));
                };
                let class = instance.borrow().class.clone();
                if class
                    .fields
                    .iter()
                    .any(|field| field.name == *name && field.private)
                    && !self.can_access(&class.name)
                {
                    return Err(self.private_error(&class.name, name, *span));
                }
                let value = instance.borrow().fields.get(name).cloned();
                value.ok_or_else(|| {
                    self.error(
                        "Name Error",
                        format!("{} has no field `{name}`", class.name),
                        *span,
                        "The requested object field does not exist.",
                        "Check the field name.",
                    )
                })
            }
        }
    }

    fn binary(&self, l: Value, op: BinaryOp, r: Value, span: Span) -> Result<Value, Diagnostic> {
        use BinaryOp::*;
        match op {
            Equal => Ok(Value::Bool(l == r)),
            NotEqual => Ok(Value::Bool(l != r)),
            And | Or => unreachable!(),
            In => match r {
                Value::List(v, _) => Ok(Value::Bool(v.borrow().contains(&l))),
                Value::Dictionary(dictionary, _) => {
                    let key = dictionary_key(&l, span, self)?;
                    Ok(Value::Bool(dictionary.borrow().position(&key).is_some()))
                }
                Value::String(s) => {
                    if let Value::String(x) = l {
                        Ok(Value::Bool(s.contains(&x)))
                    } else {
                        Err(self.type_error(span, "a String membership value", &l))
                    }
                }
                v => Err(self.type_error(span, "a List, String, or Dictionary after `in`", &v)),
            },
            Add => match (l, r) {
                (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
                (Value::List(a, _), Value::List(b, _)) => {
                    let mut v = a.borrow().clone();
                    v.extend(b.borrow().clone());
                    Ok(Value::List(Rc::new(RefCell::new(v)), false))
                }
                (a, b) => numeric2(a, b, span, self, |a, b| a + b, |a, b| a.checked_add(b)),
            },
            Subtract => numeric2(l, r, span, self, |a, b| a - b, |a, b| a.checked_sub(b)),
            Multiply => match (l, r) {
                (Value::String(s), Value::Int(n)) | (Value::Int(n), Value::String(s)) => {
                    repeat_string(s, n, span, self)
                }
                (Value::List(v, _), Value::Int(n)) | (Value::Int(n), Value::List(v, _)) => {
                    repeat_list(v, n, span, self)
                }
                (a, b) => numeric2(a, b, span, self, |a, b| a * b, |a, b| a.checked_mul(b)),
            },
            Divide => {
                let (a, b) = numbers(l, r, span, self)?;
                if b == 0.0 {
                    return Err(self.zero(span));
                }
                Ok(Value::Float(a / b))
            }
            IntegerDivide => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        Err(self.zero(span))
                    } else {
                        a.checked_div(b).map(Value::Int).ok_or_else(|| {
                            self.error(
                                "Value Error",
                                "integer overflow",
                                span,
                                "The division result is outside Int limits.",
                                "Use `/` for floating-point division.",
                            )
                        })
                    }
                }
                (a, b) => {
                    let (x, y) = numbers(a, b, span, self)?;
                    if y == 0.0 {
                        Err(self.zero(span))
                    } else {
                        Ok(Value::Float((x / y).floor()))
                    }
                }
            },
            Remainder => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        Err(self.zero(span))
                    } else {
                        a.checked_rem(b).map(Value::Int).ok_or_else(|| {
                            self.error(
                                "Value Error",
                                "integer overflow",
                                span,
                                "The remainder result is outside Int limits.",
                                "Use Float values or smaller numbers.",
                            )
                        })
                    }
                }
                (a, b) => {
                    let (x, y) = numbers(a, b, span, self)?;
                    if y == 0.0 {
                        Err(self.zero(span))
                    } else {
                        Ok(Value::Float(x % y))
                    }
                }
            },
            Power => {
                let (a, b) = numbers(l, r, span, self)?;
                let result = a.powf(b);
                if result.is_nan() {
                    Err(self.error(
                        "Math Error",
                        "power produced an invalid result",
                        span,
                        "The base and exponent are outside the real-number domain.",
                        "Use values that produce a real result.",
                    ))
                } else {
                    Ok(Value::Float(result))
                }
            }
            Less | LessEqual | Greater | GreaterEqual => {
                let ord = compare(&l, &r).ok_or_else(|| {
                    self.error(
                        "Type Error",
                        "values cannot be ordered",
                        span,
                        format!(
                            "{} and {} do not have a common ordering.",
                            l.type_name(),
                            r.type_name()
                        ),
                        "Compare two numbers or two strings.",
                    )
                })?;
                Ok(Value::Bool(match op {
                    Less => ord == Ordering::Less,
                    LessEqual => ord != Ordering::Greater,
                    Greater => ord == Ordering::Greater,
                    GreaterEqual => ord != Ordering::Less,
                    _ => false,
                }))
            }
        }
    }

    fn compound_value(
        &self,
        left: Value,
        operation: AssignOp,
        right: Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.binary(
            left,
            match operation {
                AssignOp::Add => BinaryOp::Add,
                AssignOp::Subtract => BinaryOp::Subtract,
                AssignOp::Multiply => BinaryOp::Multiply,
                AssignOp::Divide => BinaryOp::Divide,
                AssignOp::Set => unreachable!(),
            },
            right,
            span,
        )
    }

    fn call_named(&mut self, n: &str, args: Vec<Value>, span: Span) -> Result<Value, Diagnostic> {
        let value = self.get(n).ok_or_else(|| self.unknown(n, span))?.value;
        self.call_value(value, args, span)
    }
    fn call_value(&mut self, v: Value, args: Vec<Value>, span: Span) -> Result<Value, Diagnostic> {
        match v {
            Value::Function(function) => self.call_function(function, args, None, None, span),
            Value::Class(class) => self.instantiate(class, args, span),
            value => Err(self.type_error(span, "a Function or Class", &value)),
        }
    }

    fn call_function(
        &mut self,
        f: Rc<FunctionValue>,
        args: Vec<Value>,
        receiver: Option<Value>,
        owner: Option<String>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if args.len() != f.params.len() {
            return Err(self.error(
                "Argument Error",
                format!(
                    "expected {} arguments, received {}",
                    f.params.len(),
                    args.len()
                ),
                span,
                "The call does not match the function parameters.",
                "Pass the required number of arguments.",
            ));
        }
        for (parameter, value) in f.params.iter().zip(&args) {
            if let Some(ty) = &parameter.ty {
                self.ensure_type(value, ty, parameter.span)?;
            }
        }
        if self.call_depth >= 1000 {
            return Err(self.error(
                "Runtime Error",
                "maximum call depth exceeded",
                span,
                "The function calls are too deeply recursive.",
                "Add a stopping condition.",
            ));
        }
        self.call_depth += 1;
        let result = if receiver.is_none() && owner.is_none() {
            if let Some(numeric) = &f.numeric {
                self.call_numeric_function(numeric, &args)
            } else {
                self.call_ast_function(&f, args, receiver, owner, span)
            }
        } else {
            self.call_ast_function(&f, args, receiver, owner, span)
        };
        self.call_depth -= 1;
        let result = result?;
        if let Some(ty) = &f.return_type {
            self.ensure_type(&result, ty, span)?
        }
        Ok(result)
    }

    fn call_ast_function(
        &mut self,
        function: &FunctionValue,
        args: Vec<Value>,
        receiver: Option<Value>,
        owner: Option<String>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let frame_base = self.scopes.len();
        self.push();
        self.frame_bases.push(frame_base);
        let has_owner = owner.is_some();
        if let Some(owner) = owner {
            self.class_stack.push(owner);
        }
        let execution = (|| {
            if let Some(receiver) = receiver {
                self.define("self", receiver, None, true, span)?;
            }
            for (parameter, value) in function.params.iter().zip(args) {
                self.define(
                    &parameter.name,
                    value,
                    parameter.ty.clone(),
                    false,
                    parameter.span,
                )?;
            }
            let mut result = Value::None;
            for statement in &function.body.statements {
                match self.exec(statement)? {
                    Flow::Normal => {}
                    Flow::Return(value) => {
                        result = value;
                        break;
                    }
                    Flow::Break | Flow::Continue => {
                        return Err(self.error(
                            "Control Error",
                            "loop control used outside a loop",
                            statement.span(),
                            "break and continue only make sense inside loop.",
                            "Move it into a loop.",
                        ));
                    }
                }
            }
            Ok(result)
        })();
        self.pop();
        self.frame_bases.pop();
        if has_owner {
            self.class_stack.pop();
        }
        execution
    }

    fn call_numeric_function(
        &mut self,
        function: &NumericFunction,
        args: &[Value],
    ) -> Result<Value, Diagnostic> {
        let mut frame = self.numeric_frame_pool.pop().unwrap_or_default();
        frame.prepare(function);
        for (parameter, value) in function.parameters.iter().zip(args) {
            match (parameter, value) {
                (NumericParameter::Int(slot), Value::Int(value)) => frame.integers[*slot] = *value,
                (NumericParameter::Float(slot), Value::Float(value)) => {
                    frame.floats[*slot] = *value
                }
                (NumericParameter::Bool(slot), Value::Bool(value)) => frame.bools[*slot] = *value,
                _ => unreachable!("numeric function parameters were checked before execution"),
            }
        }
        let execution = self.exec_numeric_statements(&function.statements, &mut frame);
        self.numeric_frame_pool.push(frame);
        match execution? {
            NumericFlow::Normal => Ok(Value::None),
            NumericFlow::Return(value) => Ok(value),
            NumericFlow::Break | NumericFlow::Continue => {
                unreachable!("numeric compiler rejects loop control outside loops")
            }
        }
    }

    #[inline(always)]
    fn exec_numeric_statements(
        &mut self,
        statements: &[NumericStmt],
        frame: &mut NumericFrame,
    ) -> Result<NumericFlow, Diagnostic> {
        for statement in statements {
            match statement {
                NumericStmt::SetInt(slot, expression) => {
                    frame.integers[*slot] = self.eval_numeric_int(expression, frame)?;
                }
                NumericStmt::SetFloat(slot, expression) => {
                    frame.floats[*slot] = self.eval_numeric_float(expression, frame)?;
                }
                NumericStmt::SetBool(slot, expression) => {
                    frame.bools[*slot] = self.eval_numeric_bool(expression, frame)?;
                }
                NumericStmt::EvalInt(expression) => {
                    self.eval_numeric_int(expression, frame)?;
                }
                NumericStmt::EvalFloat(expression) => {
                    self.eval_numeric_float(expression, frame)?;
                }
                NumericStmt::EvalBool(expression) => {
                    self.eval_numeric_bool(expression, frame)?;
                }
                NumericStmt::SetIntList(slot, expressions) => {
                    let mut values = Vec::with_capacity(expressions.len());
                    for expression in expressions {
                        values.push(self.eval_numeric_int(expression, frame)?);
                    }
                    frame.int_lists[*slot] = values;
                }
                NumericStmt::SetFloatList(slot, expressions) => {
                    let mut values = Vec::with_capacity(expressions.len());
                    for expression in expressions {
                        values.push(self.eval_numeric_float(expression, frame)?);
                    }
                    frame.float_lists[*slot] = values;
                }
                NumericStmt::AddIntList(slot, expressions) => {
                    let mut values = Vec::with_capacity(expressions.len());
                    for expression in expressions {
                        values.push(self.eval_numeric_int(expression, frame)?);
                    }
                    frame.int_lists[*slot].extend(values);
                }
                NumericStmt::AddFloatList(slot, expressions) => {
                    let mut values = Vec::with_capacity(expressions.len());
                    for expression in expressions {
                        values.push(self.eval_numeric_float(expression, frame)?);
                    }
                    frame.float_lists[*slot].extend(values);
                }
                NumericStmt::SetIntListIndex(slot, index, value, span) => {
                    let index = self.eval_numeric_int(index, frame)?;
                    let value = self.eval_numeric_int(value, frame)?;
                    let len = frame.int_lists[*slot].len();
                    let Some(index) = normalize_index(index, len) else {
                        return Err(self.index_error(*span, index, len));
                    };
                    frame.int_lists[*slot][index] = value;
                }
                NumericStmt::SetFloatListIndex(slot, index, value, span) => {
                    let index = self.eval_numeric_int(index, frame)?;
                    let value = self.eval_numeric_float(value, frame)?;
                    let len = frame.float_lists[*slot].len();
                    let Some(index) = normalize_index(index, len) else {
                        return Err(self.index_error(*span, index, len));
                    };
                    frame.float_lists[*slot][index] = value;
                }
                NumericStmt::SortIntList(slot) => frame.int_lists[*slot].sort_unstable(),
                NumericStmt::SortFloatList(slot) => frame.float_lists[*slot]
                    .sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal)),
                NumericStmt::ReverseIntList(slot) => frame.int_lists[*slot].reverse(),
                NumericStmt::ReverseFloatList(slot) => frame.float_lists[*slot].reverse(),
                NumericStmt::ClearIntList(slot) => frame.int_lists[*slot].clear(),
                NumericStmt::ClearFloatList(slot) => frame.float_lists[*slot].clear(),
                NumericStmt::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let branch = if self.eval_numeric_bool(condition, frame)? {
                        then_body
                    } else {
                        else_body
                    };
                    match self.exec_numeric_statements(branch, frame)? {
                        NumericFlow::Normal => {}
                        flow => return Ok(flow),
                    }
                }
                NumericStmt::Loop {
                    condition,
                    body,
                    span,
                } => {
                    let mut iterations = 0usize;
                    loop {
                        if let Some(condition) = condition {
                            if !self.eval_numeric_bool(condition, frame)? {
                                break;
                            }
                        }
                        if iterations >= MAX_LOOP_ITERATIONS {
                            return Err(self.error(
                                "Runtime Error",
                                "loop iteration limit exceeded",
                                *span,
                                "The loop ran more than ten million times.",
                                "Check that the loop can finish.",
                            ));
                        }
                        iterations += 1;
                        match self.exec_numeric_statements(body, frame)? {
                            NumericFlow::Normal | NumericFlow::Continue => {}
                            NumericFlow::Break => break,
                            NumericFlow::Return(value) => {
                                return Ok(NumericFlow::Return(value));
                            }
                        }
                    }
                }
                NumericStmt::ForRange {
                    variable,
                    start,
                    end,
                    step,
                    body,
                    span,
                } => {
                    let mut value = self.eval_numeric_int(start, frame)?;
                    let mut iterations = 0usize;
                    let end = self.eval_numeric_int(end, frame)?;
                    let step = if let Some(step) = step {
                        self.eval_numeric_int(step, frame)?
                    } else if value <= end {
                        1
                    } else {
                        -1
                    };
                    if step == 0 {
                        return Err(self.error(
                            "Value Error",
                            "range step cannot be zero",
                            *span,
                            "A zero step never advances.",
                            "Use a positive or negative integer step.",
                        ));
                    }
                    while if step > 0 { value < end } else { value > end } {
                        if iterations >= MAX_LOOP_ITERATIONS {
                            return Err(self.error(
                                "Runtime Error",
                                "loop iteration limit exceeded",
                                *span,
                                "The loop ran more than ten million times.",
                                "Use a smaller range or split the work into batches.",
                            ));
                        }
                        iterations += 1;
                        frame.integers[*variable] = value;
                        match self.exec_numeric_statements(body, frame)? {
                            NumericFlow::Normal | NumericFlow::Continue => {}
                            NumericFlow::Break => break,
                            NumericFlow::Return(value) => {
                                return Ok(NumericFlow::Return(value));
                            }
                        }
                        value = value.checked_add(step).ok_or_else(|| {
                            self.error(
                                "Value Error",
                                "range overflow",
                                *span,
                                "The range exceeded Int limits.",
                                "Use a smaller range.",
                            )
                        })?;
                    }
                }
                NumericStmt::ForIntList {
                    variable,
                    list,
                    body,
                    span,
                } => {
                    let values = frame.int_lists[*list].clone();
                    for (iterations, value) in values.into_iter().enumerate() {
                        if iterations >= MAX_LOOP_ITERATIONS {
                            return Err(self.error(
                                "Runtime Error",
                                "loop iteration limit exceeded",
                                *span,
                                "The loop ran more than ten million times.",
                                "Use a smaller iterable or split the work into batches.",
                            ));
                        }
                        frame.integers[*variable] = value;
                        match self.exec_numeric_statements(body, frame)? {
                            NumericFlow::Normal | NumericFlow::Continue => {}
                            NumericFlow::Break => break,
                            NumericFlow::Return(value) => {
                                return Ok(NumericFlow::Return(value));
                            }
                        }
                    }
                }
                NumericStmt::ForFloatList {
                    variable,
                    list,
                    body,
                    span,
                } => {
                    let values = frame.float_lists[*list].clone();
                    for (iterations, value) in values.into_iter().enumerate() {
                        if iterations >= MAX_LOOP_ITERATIONS {
                            return Err(self.error(
                                "Runtime Error",
                                "loop iteration limit exceeded",
                                *span,
                                "The loop ran more than ten million times.",
                                "Use a smaller iterable or split the work into batches.",
                            ));
                        }
                        frame.floats[*variable] = value;
                        match self.exec_numeric_statements(body, frame)? {
                            NumericFlow::Normal | NumericFlow::Continue => {}
                            NumericFlow::Break => break,
                            NumericFlow::Return(value) => {
                                return Ok(NumericFlow::Return(value));
                            }
                        }
                    }
                }
                NumericStmt::Break => return Ok(NumericFlow::Break),
                NumericStmt::Continue => return Ok(NumericFlow::Continue),
                NumericStmt::Return(value) => {
                    let value = match value {
                        Some(value) => self.eval_numeric_value(value, frame)?,
                        None => Value::None,
                    };
                    return Ok(NumericFlow::Return(value));
                }
            }
        }
        Ok(NumericFlow::Normal)
    }

    #[inline(always)]
    fn eval_numeric_int(
        &mut self,
        expression: &VmIntExpr,
        frame: &NumericFrame,
    ) -> Result<i64, Diagnostic> {
        match expression {
            VmIntExpr::Literal(value) => Ok(*value),
            VmIntExpr::Local(slot) => Ok(frame.integers[*slot]),
            VmIntExpr::Global(name, span) => match self.get_cached(name) {
                Some(Value::Int(value)) => Ok(value),
                Some(value) => Err(self.type_error(*span, "an Int", &value)),
                None => Err(self.unknown(name, *span)),
            },
            VmIntExpr::Negate(value, span) => self
                .eval_numeric_int(value, frame)?
                .checked_neg()
                .ok_or_else(|| integer_overflow(*span, self.file, self.source)),
            VmIntExpr::Add(left, right, span) => self
                .eval_numeric_int(left, frame)?
                .checked_add(self.eval_numeric_int(right, frame)?)
                .ok_or_else(|| integer_overflow(*span, self.file, self.source)),
            VmIntExpr::Subtract(left, right, span) => self
                .eval_numeric_int(left, frame)?
                .checked_sub(self.eval_numeric_int(right, frame)?)
                .ok_or_else(|| integer_overflow(*span, self.file, self.source)),
            VmIntExpr::Multiply(left, right, span) => self
                .eval_numeric_int(left, frame)?
                .checked_mul(self.eval_numeric_int(right, frame)?)
                .ok_or_else(|| integer_overflow(*span, self.file, self.source)),
            VmIntExpr::IntegerDivide(left, right, span) => {
                let left = self.eval_numeric_int(left, frame)?;
                let right = self.eval_numeric_int(right, frame)?;
                if right == 0 {
                    return Err(self.zero(*span));
                }
                left.checked_div(right)
                    .ok_or_else(|| integer_overflow(*span, self.file, self.source))
            }
            VmIntExpr::Remainder(left, right, span) => {
                let left = self.eval_numeric_int(left, frame)?;
                let right = self.eval_numeric_int(right, frame)?;
                if right == 0 {
                    return Err(self.zero(*span));
                }
                left.checked_rem(right)
                    .ok_or_else(|| integer_overflow(*span, self.file, self.source))
            }
            VmIntExpr::ListIndex(slot, index, span) => {
                let index = self.eval_numeric_int(index, frame)?;
                let values = &frame.int_lists[*slot];
                let Some(index) = normalize_index(index, values.len()) else {
                    return Err(self.index_error(*span, index, values.len()));
                };
                Ok(values[index])
            }
            VmIntExpr::ListLength(list) => Ok(match list {
                NumericListRef::Int(slot) => frame.int_lists[*slot].len() as i64,
                NumericListRef::Float(slot) => frame.float_lists[*slot].len() as i64,
            }),
            VmIntExpr::ListAggregate(slot, operation, span) => {
                self.eval_numeric_int_aggregate(*operation, &frame.int_lists[*slot], *span)
            }
            VmIntExpr::Call(call) => match self.eval_numeric_call(call, frame)? {
                Value::Int(value) => Ok(value),
                value => Err(self.type_error(call.span, "an Int", &value)),
            },
        }
    }

    #[inline(always)]
    fn eval_numeric_float(
        &mut self,
        expression: &VmFloatExpr,
        frame: &NumericFrame,
    ) -> Result<f64, Diagnostic> {
        match expression {
            VmFloatExpr::Literal(value) => Ok(*value),
            VmFloatExpr::Local(slot) => Ok(frame.floats[*slot]),
            VmFloatExpr::Global(name, span) => match self.get_cached(name) {
                Some(Value::Float(value)) => Ok(value),
                Some(Value::Int(value)) => Ok(value as f64),
                Some(value) => Err(self.type_error(*span, "a Number", &value)),
                None => Err(self.unknown(name, *span)),
            },
            VmFloatExpr::FromInt(value) => Ok(self.eval_numeric_int(value, frame)? as f64),
            VmFloatExpr::Negate(value) => Ok(-self.eval_numeric_float(value, frame)?),
            VmFloatExpr::Add(left, right) => Ok(
                self.eval_numeric_float(left, frame)? + self.eval_numeric_float(right, frame)?
            ),
            VmFloatExpr::Subtract(left, right) => Ok(
                self.eval_numeric_float(left, frame)? - self.eval_numeric_float(right, frame)?
            ),
            VmFloatExpr::Multiply(left, right) => Ok(
                self.eval_numeric_float(left, frame)? * self.eval_numeric_float(right, frame)?
            ),
            VmFloatExpr::Divide(left, right, span) => {
                let left = self.eval_numeric_float(left, frame)?;
                let right = self.eval_numeric_float(right, frame)?;
                if right == 0.0 {
                    return Err(self.zero(*span));
                }
                Ok(left / right)
            }
            VmFloatExpr::IntegerDivide(left, right, span) => {
                let left = self.eval_numeric_float(left, frame)?;
                let right = self.eval_numeric_float(right, frame)?;
                if right == 0.0 {
                    return Err(self.zero(*span));
                }
                Ok((left / right).floor())
            }
            VmFloatExpr::Remainder(left, right, span) => {
                let left = self.eval_numeric_float(left, frame)?;
                let right = self.eval_numeric_float(right, frame)?;
                if right == 0.0 {
                    return Err(self.zero(*span));
                }
                Ok(left % right)
            }
            VmFloatExpr::Power(left, right, span) => {
                let output = self
                    .eval_numeric_float(left, frame)?
                    .powf(self.eval_numeric_float(right, frame)?);
                if output.is_nan() {
                    Err(self.error(
                        "Math Error",
                        "power produced an invalid result",
                        *span,
                        "The base and exponent are outside the real-number domain.",
                        "Use values that produce a real result.",
                    ))
                } else {
                    Ok(output)
                }
            }
            VmFloatExpr::Math(operation, value, span) => {
                let value = self.eval_numeric_float(value, frame)?;
                let output = operation.apply(value);
                if output.is_nan() {
                    Err(self.error(
                        "Math Error",
                        format!("{} produced an invalid result", operation.name()),
                        *span,
                        "The input is outside the real-number domain.",
                        "Use a value in the function's valid domain.",
                    ))
                } else {
                    Ok(output)
                }
            }
            VmFloatExpr::ListIndex(slot, index, span) => {
                let index = self.eval_numeric_int(index, frame)?;
                let values = &frame.float_lists[*slot];
                let Some(index) = normalize_index(index, values.len()) else {
                    return Err(self.index_error(*span, index, values.len()));
                };
                Ok(values[index])
            }
            VmFloatExpr::ListAggregate(list, operation, span) => {
                self.eval_numeric_float_aggregate(*list, *operation, frame, *span)
            }
            VmFloatExpr::Call(call) => match self.eval_numeric_call(call, frame)? {
                Value::Float(value) => Ok(value),
                value => Err(self.type_error(call.span, "a Float", &value)),
            },
        }
    }

    #[inline(always)]
    fn eval_numeric_bool(
        &mut self,
        expression: &VmBoolExpr,
        frame: &NumericFrame,
    ) -> Result<bool, Diagnostic> {
        match expression {
            VmBoolExpr::Literal(value) => Ok(*value),
            VmBoolExpr::Local(slot) => Ok(frame.bools[*slot]),
            VmBoolExpr::Global(name, span) => match self.get_cached(name) {
                Some(Value::Bool(value)) => Ok(value),
                Some(value) => Err(self.type_error(*span, "a Bool", &value)),
                None => Err(self.unknown(name, *span)),
            },
            VmBoolExpr::Not(value) => Ok(!self.eval_numeric_bool(value, frame)?),
            VmBoolExpr::And(left, right) => {
                if !self.eval_numeric_bool(left, frame)? {
                    Ok(false)
                } else {
                    self.eval_numeric_bool(right, frame)
                }
            }
            VmBoolExpr::Or(left, right) => {
                if self.eval_numeric_bool(left, frame)? {
                    Ok(true)
                } else {
                    self.eval_numeric_bool(right, frame)
                }
            }
            VmBoolExpr::IntTruthy(value) => Ok(self.eval_numeric_int(value, frame)? != 0),
            VmBoolExpr::FloatTruthy(value) => Ok(self.eval_numeric_float(value, frame)? != 0.0),
            VmBoolExpr::NumberCompare(left, operation, right, span) => {
                let left = self.eval_numeric_number(left, frame)?;
                let right = self.eval_numeric_number(right, frame)?;
                self.numeric_comparison(left, *operation, right, *span)
            }
            VmBoolExpr::BoolCompare(left, operation, right) => {
                let left = self.eval_numeric_bool(left, frame)?;
                let right = self.eval_numeric_bool(right, frame)?;
                Ok(comparison_matches(left.cmp(&right), *operation))
            }
            VmBoolExpr::Call(call) => match self.eval_numeric_call(call, frame)? {
                Value::Bool(value) => Ok(value),
                value => Err(self.type_error(call.span, "a Bool", &value)),
            },
        }
    }

    fn eval_numeric_number(
        &mut self,
        expression: &numeric_vm::NumberExpr,
        frame: &NumericFrame,
    ) -> Result<NumericNumber, Diagnostic> {
        match expression {
            numeric_vm::NumberExpr::Int(value) => {
                Ok(NumericNumber::Int(self.eval_numeric_int(value, frame)?))
            }
            numeric_vm::NumberExpr::Float(value) => {
                Ok(NumericNumber::Float(self.eval_numeric_float(value, frame)?))
            }
        }
    }

    fn eval_numeric_value(
        &mut self,
        expression: &NumericValueExpr,
        frame: &NumericFrame,
    ) -> Result<Value, Diagnostic> {
        match expression {
            NumericValueExpr::Int(value) => Ok(Value::Int(self.eval_numeric_int(value, frame)?)),
            NumericValueExpr::Float(value) => {
                Ok(Value::Float(self.eval_numeric_float(value, frame)?))
            }
            NumericValueExpr::Bool(value) => Ok(Value::Bool(self.eval_numeric_bool(value, frame)?)),
        }
    }

    fn eval_numeric_call(
        &mut self,
        call: &NumericCall,
        frame: &NumericFrame,
    ) -> Result<Value, Diagnostic> {
        let mut args = Vec::with_capacity(call.args.len());
        for argument in &call.args {
            args.push(self.eval_numeric_value(argument, frame)?);
        }
        if call.builtin {
            self.call_builtin(&call.name, args, call.span)
        } else {
            self.call_named(&call.name, args, call.span)
        }
    }

    fn numeric_comparison(
        &self,
        left: NumericNumber,
        operation: VmCompareOp,
        right: NumericNumber,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        if matches!(operation, VmCompareOp::Equal | VmCompareOp::NotEqual) {
            let equal = numeric_numbers_equal(left, right);
            return Ok(if matches!(operation, VmCompareOp::Equal) {
                equal
            } else {
                !equal
            });
        }
        let ordering = numeric_number_cmp(left, right).ok_or_else(|| {
            self.error(
                "Type Error",
                "values cannot be ordered",
                span,
                "NAN does not have a mathematical ordering.",
                "Check for NAN before ordering values.",
            )
        })?;
        Ok(comparison_matches(ordering, operation))
    }

    fn eval_numeric_int_aggregate(
        &self,
        operation: IntListOp,
        values: &[i64],
        span: Span,
    ) -> Result<i64, Diagnostic> {
        if values.is_empty() && !matches!(operation, IntListOp::Sum | IntListOp::Product) {
            return Err(self.error(
                "Value Error",
                "aggregate requires a non-empty List",
                span,
                "There is no aggregate value for an empty list.",
                "Add numeric values first.",
            ));
        }
        match operation {
            IntListOp::Sum => values.iter().try_fold(0i64, |total, value| {
                total.checked_add(*value).ok_or_else(|| {
                    self.error(
                        "Value Error",
                        "integer overflow in sum",
                        span,
                        "The sum exceeds Int limits.",
                        "Use Float values or smaller numbers.",
                    )
                })
            }),
            IntListOp::Product => values.iter().try_fold(1i64, |total, value| {
                total.checked_mul(*value).ok_or_else(|| {
                    self.error(
                        "Value Error",
                        "integer overflow in product",
                        span,
                        "The product exceeds Int limits.",
                        "Use Float values or smaller numbers.",
                    )
                })
            }),
            IntListOp::Min => Ok(*values.iter().min().unwrap()),
            IntListOp::Max => Ok(*values.iter().max().unwrap()),
        }
    }

    fn eval_numeric_float_aggregate(
        &self,
        list: NumericListRef,
        operation: FloatListOp,
        frame: &NumericFrame,
        span: Span,
    ) -> Result<f64, Diagnostic> {
        match list {
            NumericListRef::Int(slot) => self.float_aggregate(
                operation,
                frame.int_lists[slot].iter().map(|value| *value as f64),
                frame.int_lists[slot].len(),
                span,
            ),
            NumericListRef::Float(slot) => self.float_aggregate(
                operation,
                frame.float_lists[slot].iter().copied(),
                frame.float_lists[slot].len(),
                span,
            ),
        }
    }

    fn float_aggregate<I>(
        &self,
        operation: FloatListOp,
        values: I,
        len: usize,
        span: Span,
    ) -> Result<f64, Diagnostic>
    where
        I: Iterator<Item = f64> + Clone,
    {
        if len == 0 && !matches!(operation, FloatListOp::Sum | FloatListOp::Product) {
            return Err(self.error(
                "Value Error",
                "aggregate requires a non-empty List",
                span,
                "There is no aggregate value for an empty list.",
                "Add numeric values first.",
            ));
        }
        match operation {
            FloatListOp::Sum => Ok(compensated_sum(values)),
            FloatListOp::Product => Ok(values.product()),
            FloatListOp::Min | FloatListOp::Max => {
                let mut values = values;
                let mut best = values.next().unwrap();
                for value in values {
                    let ordering = value.partial_cmp(&best).unwrap_or(Ordering::Equal);
                    if (matches!(operation, FloatListOp::Min) && ordering == Ordering::Less)
                        || (matches!(operation, FloatListOp::Max) && ordering == Ordering::Greater)
                    {
                        best = value;
                    }
                }
                Ok(best)
            }
            FloatListOp::Mean => Ok(compensated_sum(values) / len as f64),
            FloatListOp::Variance | FloatListOp::Std => {
                let mut count = 0.0;
                let mut mean = 0.0;
                let mut squared = 0.0;
                for value in values {
                    count += 1.0;
                    let delta = value - mean;
                    mean += delta / count;
                    squared += delta * (value - mean);
                }
                let variance = squared / count;
                Ok(if matches!(operation, FloatListOp::Std) {
                    variance.sqrt()
                } else {
                    variance
                })
            }
            FloatListOp::Median => {
                let mut sorted = values.collect::<Vec<_>>();
                sorted.sort_by(f64::total_cmp);
                let middle = sorted.len() / 2;
                Ok(if sorted.len() % 2 == 0 {
                    (sorted[middle - 1] + sorted[middle]) / 2.0
                } else {
                    sorted[middle]
                })
            }
            FloatListOp::Mode => {
                let mut sorted = values.collect::<Vec<_>>();
                sorted.sort_by(f64::total_cmp);
                let mut best = sorted[0];
                let mut best_count = 1usize;
                let mut current = sorted[0];
                let mut current_count = 1usize;
                for value in sorted.into_iter().skip(1) {
                    if value.total_cmp(&current) == Ordering::Equal {
                        current_count += 1;
                    } else {
                        if current_count > best_count {
                            best = current;
                            best_count = current_count;
                        }
                        current = value;
                        current_count = 1;
                    }
                }
                if current_count > best_count {
                    best = current;
                }
                Ok(best)
            }
        }
    }

    fn instantiate(
        &mut self,
        class: Rc<ClassValue>,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let mut fields = HashMap::default();
        for field in &class.fields {
            fields.insert(field.name.clone(), self.eval(&field.value)?);
        }
        let instance = Value::Instance(
            Rc::new(RefCell::new(InstanceValue {
                class: class.clone(),
                fields,
            })),
            false,
        );
        if let Some(initializer) = class.methods.get("init") {
            self.call_function(
                initializer.function.clone(),
                args,
                Some(instance.clone()),
                Some(class.name.clone()),
                span,
            )?;
        } else if !args.is_empty() {
            return Err(self.arg_error(span, &class.name, 0, args.len()));
        }
        Ok(instance)
    }

    fn member(
        &mut self,
        obj: Value,
        name: &str,
        args: Vec<Value>,
        expected_item: Option<&TypeRef>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if let Value::Instance(instance, frozen) = &obj {
            let class = instance.borrow().class.clone();
            let method = class.methods.get(name).cloned().ok_or_else(|| {
                self.error(
                    "Name Error",
                    format!("{} has no method `{name}`", class.name),
                    span,
                    "The requested object method does not exist.",
                    "Check the method name.",
                )
            })?;
            if method.private && !self.can_access(&class.name) {
                return Err(self.private_error(&class.name, name, span));
            }
            return self.call_function(
                method.function,
                args,
                Some(Value::Instance(instance.clone(), *frozen)),
                Some(class.name.clone()),
                span,
            );
        }
        if let Value::Dictionary(dictionary, frozen) = &obj {
            return self.dictionary_member(dictionary.clone(), *frozen, name, args, span);
        }
        let Value::List(items, frozen) = obj else {
            return Err(self.type_error(
                span,
                "a List, Dictionary, or Object before the method",
                &obj,
            ));
        };
        if name == "add" && !frozen && args.iter().any(|value| list_would_cycle(&items, value)) {
            return Err(self.error(
                "Value Error",
                "cannot create a cyclic List",
                span,
                "Cyclic Lists make copying and comparison unsafe.",
                "Store a scalar value or a separate copy instead.",
            ));
        }
        let mut v = items.borrow_mut();
        match name{
        "add"=>{
            if frozen{return Err(self.const_error(span))}
            if args.is_empty(){return Err(self.arg_error(span,"add",1,args.len()))}
            if let Some(expected) = expected_item {
                for value in &args {
                    self.ensure_type(value, expected, span)?;
                }
            }
            v.extend(args);Ok(Value::None)
        },
        "del"=>{if frozen{return Err(self.const_error(span))}one_arg(&args,span,"del",self)?;let i=as_int(args[0].clone(),span,self)?;let i=normalize_index(i,v.len()).ok_or_else(||self.index_error(span,i,v.len()))?;Ok(v.remove(i))},
        "remove"=>{if frozen{return Err(self.const_error(span))}one_arg(&args,span,"remove",self)?;if let Some(i)=v.iter().position(|x|x==&args[0]){v.remove(i);Ok(Value::Bool(true))}else{Ok(Value::Bool(false))}},
        "have"=>{one_arg(&args,span,"have",self)?;Ok(Value::Bool(v.contains(&args[0])))},
        "index"=>{one_arg(&args,span,"index",self)?;Ok(v.iter().position(|x|x==&args[0]).map(|i|Value::Int(i as i64)).unwrap_or(Value::Bool(false)))},
        "len"=>{zero_args(&args,span,"len",self)?;Ok(Value::Int(v.len() as i64))},
        "clear"=>{zero_args(&args,span,"clear",self)?;if frozen{return Err(self.const_error(span))}v.clear();Ok(Value::None)},
        "copy"=>{zero_args(&args,span,"copy",self)?;Ok(Value::List(Rc::new(RefCell::new(v.clone())),false))},
        "unique"=>{zero_args(&args,span,"unique",self)?;Ok(Value::List(Rc::new(RefCell::new(unique_values(&v))),false))},
        "reverse"=>{zero_args(&args,span,"reverse",self)?;if frozen{return Err(self.const_error(span))}v.reverse();Ok(Value::None)},
        "sort"=>{zero_args(&args,span,"sort",self)?;if frozen{return Err(self.const_error(span))}v.sort_by(|a,b|compare(a,b).unwrap_or(Ordering::Equal));Ok(Value::None)},
        "sum"|"product"|"min"|"max"|"mean"|"median"|"mode"|"variance"|"std"=>{zero_args(&args,span,name,self)?;aggregate(name,&v,span,self)},
        _=>Err(self.error("Name Error",format!("List has no method `{name}`"),span,"The requested list method does not exist.","Use add, del, remove, have, index, len, clear, copy, unique, reverse, sort, sum, min, max, or mean.")),
    }
    }

    fn dictionary_member(
        &self,
        dictionary: Rc<RefCell<DictionaryValue>>,
        frozen: bool,
        name: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let expected_arguments = match name {
            "have" | "remove" => Some((1, 1)),
            "get" => Some((1, 2)),
            "len" | "clear" | "copy" | "keys" | "values" | "items" => Some((0, 0)),
            _ => None,
        }
        .ok_or_else(|| {
            self.error(
                "Name Error",
                format!("Dictionary has no method `{name}`"),
                span,
                "The requested Dictionary method does not exist.",
                "Use have, get, remove, len, clear, copy, keys, values, or items.",
            )
        })?;
        if args.len() < expected_arguments.0 || args.len() > expected_arguments.1 {
            let expected = if expected_arguments.0 == expected_arguments.1 {
                expected_arguments.0.to_string()
            } else {
                format!("{} or {}", expected_arguments.0, expected_arguments.1)
            };
            return Err(self.error(
                "Argument Error",
                format!(
                    "{name} expects {expected} argument(s), received {}",
                    args.len()
                ),
                span,
                "The Dictionary method has the wrong number of arguments.",
                "Adjust the arguments to match the method.",
            ));
        }

        if let Some(key) = args.first() {
            let expected = dictionary.borrow().key_type.clone();
            if let Some(expected) = &expected {
                self.ensure_type(key, expected, span)?;
            }
            dictionary_key(key, span, self)?;
        }

        match name {
            "have" => {
                let key = dictionary_key(&args[0], span, self)?;
                Ok(Value::Bool(dictionary.borrow().position(&key).is_some()))
            }
            "get" => {
                let key = dictionary_key(&args[0], span, self)?;
                let dictionary = dictionary.borrow();
                Ok(dictionary
                    .position(&key)
                    .map(|position| dictionary.entries[position].value.clone())
                    .unwrap_or_else(|| args.get(1).cloned().unwrap_or(Value::None)))
            }
            "remove" => {
                if frozen {
                    return Err(self.const_error(span));
                }
                let key = dictionary_key(&args[0], span, self)?;
                Ok(Value::Bool(dictionary.borrow_mut().remove(&key)))
            }
            "len" => Ok(Value::Int(dictionary.borrow().entries.len() as i64)),
            "clear" => {
                if frozen {
                    return Err(self.const_error(span));
                }
                dictionary.borrow_mut().clear();
                Ok(Value::None)
            }
            "copy" => {
                let dictionary = dictionary.borrow();
                Ok(Value::Dictionary(
                    Rc::new(RefCell::new(DictionaryValue {
                        entries: dictionary.entries.clone(),
                        index: dictionary.index.clone(),
                        key_type: dictionary.key_type.clone(),
                        value_type: dictionary.value_type.clone(),
                    })),
                    false,
                ))
            }
            "keys" => Ok(Value::List(
                Rc::new(RefCell::new(
                    dictionary
                        .borrow()
                        .entries
                        .iter()
                        .map(|entry| entry.key.clone())
                        .collect(),
                )),
                false,
            )),
            "values" => Ok(Value::List(
                Rc::new(RefCell::new(
                    dictionary
                        .borrow()
                        .entries
                        .iter()
                        .map(|entry| entry.value.clone())
                        .collect(),
                )),
                false,
            )),
            "items" => Ok(Value::List(
                Rc::new(RefCell::new(
                    dictionary
                        .borrow()
                        .entries
                        .iter()
                        .map(|entry| {
                            Value::List(
                                Rc::new(RefCell::new(vec![entry.key.clone(), entry.value.clone()])),
                                false,
                            )
                        })
                        .collect(),
                )),
                false,
            )),
            _ => unreachable!(),
        }
    }

    fn call_builtin(&mut self, n: &str, a: Vec<Value>, span: Span) -> Result<Value, Diagnostic> {
        match n {
            "assert" => {
                if a.is_empty() || a.len() > 2 {
                    return Err(self.error(
                        "Argument Error",
                        "assert expects one or two arguments",
                        span,
                        "Use assert(condition) or assert(condition, message).",
                        "Pass a condition and optional message.",
                    ));
                }
                if !a[0].truthy() {
                    let message = a
                        .get(1)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "assertion failed".into());
                    return Err(self.error(
                        "Assertion Error",
                        message,
                        span,
                        "A test condition evaluated to false.",
                        "Fix the program or expected value.",
                    ));
                }
                Ok(Value::None)
            }
            "print" => {
                if a.is_empty() {
                    if self.output {
                        println!()
                    }
                    return Ok(Value::None);
                }
                if self.output {
                    for (i, v) in a.iter().enumerate() {
                        if i > 0 {
                            print!(" ")
                        }
                        print!("{v}")
                    }
                    println!()
                }
                Ok(Value::None)
            }
            "input" => {
                if a.len() > 1 {
                    return Err(self.arg_error(span, n, 1, a.len()));
                }
                if self.output {
                    if let Some(v) = a.first() {
                        print!("{v}");
                        io::stdout().flush().ok();
                    }
                    let mut s = String::new();
                    io::stdin().read_line(&mut s).map_err(|e| {
                        self.error(
                            "Input Error",
                            "could not read input",
                            span,
                            e.to_string(),
                            "Try the command again.",
                        )
                    })?;
                    Ok(Value::String(s.trim_end_matches(['\n', '\r']).to_string()))
                } else {
                    Ok(Value::String(String::new()))
                }
            }
            "length" => {
                one_arg(&a, span, n, self)?;
                match &a[0] {
                    Value::List(v, _) => Ok(Value::Int(v.borrow().len() as i64)),
                    Value::Dictionary(v, _) => Ok(Value::Int(v.borrow().entries.len() as i64)),
                    Value::String(v) => Ok(Value::Int(v.chars().count() as i64)),
                    v => Err(self.type_error(span, "a List, Dictionary, or String", v)),
                }
            }
            "type" => {
                one_arg(&a, span, n, self)?;
                Ok(Value::String(a[0].type_name().into()))
            }
            "string" => {
                one_arg(&a, span, n, self)?;
                Ok(Value::String(a[0].to_string()))
            }
            "bool" => {
                one_arg(&a, span, n, self)?;
                Ok(Value::Bool(a[0].truthy()))
            }
            "number" => {
                one_arg(&a, span, n, self)?;
                match &a[0] {
                    Value::Int(v) => Ok(Value::Int(*v)),
                    Value::Float(v) => Ok(Value::Float(*v)),
                    Value::Bool(v) => Ok(Value::Int(*v as i64)),
                    Value::String(s) => {
                        if let Ok(v) = s.parse::<i64>() {
                            Ok(Value::Int(v))
                        } else if let Ok(v) = s.parse::<f64>() {
                            Ok(Value::Float(v))
                        } else {
                            Err(self.error(
                                "Conversion Error",
                                format!("cannot convert `{s}` to Number"),
                                span,
                                "The string does not contain a valid number.",
                                "Use digits with an optional decimal point or exponent.",
                            ))
                        }
                    }
                    v => Err(self.type_error(span, "a String, Bool, or Number", v)),
                }
            }
            "readFile" => {
                one_arg(&a, span, n, self)?;
                let p = as_string(&a[0], span, self)?;
                if !self.output {
                    return Ok(Value::String(String::new()));
                }
                std::fs::read_to_string(p).map(Value::String).map_err(|e| {
                    self.error(
                        "File Error",
                        format!("could not read `{p}`"),
                        span,
                        e.to_string(),
                        "Check that the path exists and is readable.",
                    )
                })
            }
            "writeFile" => {
                if a.len() != 2 {
                    return Err(self.arg_error(span, n, 2, a.len()));
                }
                let p = as_string(&a[0], span, self)?;
                let text = as_string(&a[1], span, self)?;
                if self.output {
                    std::fs::write(p, text).map_err(|e| {
                        self.error(
                            "File Error",
                            format!("could not write `{p}`"),
                            span,
                            e.to_string(),
                            "Check the path and directory permissions.",
                        )
                    })?
                }
                Ok(Value::None)
            }
            "abs" => unary_math(n, a, span, self, |x| x.abs()),
            "floor" => unary_math(n, a, span, self, |x| x.floor()),
            "ceil" => unary_math(n, a, span, self, |x| x.ceil()),
            "sqrt" => unary_math(n, a, span, self, |x| x.sqrt()),
            "sin" => unary_math(n, a, span, self, |x| x.sin()),
            "cos" => unary_math(n, a, span, self, |x| x.cos()),
            "tan" => unary_math(n, a, span, self, |x| x.tan()),
            "asin" => unary_math(n, a, span, self, |x| x.asin()),
            "acos" => unary_math(n, a, span, self, |x| x.acos()),
            "atan" => unary_math(n, a, span, self, |x| x.atan()),
            "log" => unary_math(n, a, span, self, |x| x.ln()),
            "log10" => unary_math(n, a, span, self, |x| x.log10()),
            "log2" => unary_math(n, a, span, self, |x| x.log2()),
            "exp" => unary_math(n, a, span, self, |x| x.exp()),
            "exp2" => unary_math(n, a, span, self, |x| x.exp2()),
            "cbrt" => unary_math(n, a, span, self, |x| x.cbrt()),
            "trunc" => unary_math(n, a, span, self, |x| x.trunc()),
            "fract" => unary_math(n, a, span, self, |x| x.fract()),
            "sinh" => unary_math(n, a, span, self, |x| x.sinh()),
            "cosh" => unary_math(n, a, span, self, |x| x.cosh()),
            "tanh" => unary_math(n, a, span, self, |x| x.tanh()),
            "asinh" => unary_math(n, a, span, self, |x| x.asinh()),
            "acosh" => unary_math(n, a, span, self, |x| x.acosh()),
            "atanh" => unary_math(n, a, span, self, |x| x.atanh()),
            "degrees" => unary_math(n, a, span, self, |x| x.to_degrees()),
            "radians" => unary_math(n, a, span, self, |x| x.to_radians()),
            "hypot" => binary_math(n, a, span, self, |x, y| x.hypot(y)),
            "atan2" => binary_math(n, a, span, self, |y, x| y.atan2(x)),
            "sign" => {
                one_arg(&a, span, n, self)?;
                let value = as_number(&a[0], span, self)?;
                if value.is_nan() {
                    return Err(self.error(
                        "Math Error",
                        "sign is undefined for NAN",
                        span,
                        "NAN has no sign.",
                        "Use a finite number.",
                    ));
                }
                Ok(Value::Int(if value > 0.0 {
                    1
                } else if value < 0.0 {
                    -1
                } else {
                    0
                }))
            }
            "isNan" | "isInfinite" | "isFinite" => {
                one_arg(&a, span, n, self)?;
                let value = as_number(&a[0], span, self)?;
                Ok(Value::Bool(match n {
                    "isNan" => value.is_nan(),
                    "isInfinite" => value.is_infinite(),
                    "isFinite" => value.is_finite(),
                    _ => unreachable!(),
                }))
            }
            "clamp" => {
                if a.len() != 3 {
                    return Err(self.arg_error(span, n, 3, a.len()));
                }
                let value = as_number(&a[0], span, self)?;
                let minimum = as_number(&a[1], span, self)?;
                let maximum = as_number(&a[2], span, self)?;
                if minimum > maximum {
                    return Err(self.error(
                        "Value Error",
                        "clamp minimum exceeds maximum",
                        span,
                        "The lower bound must not exceed the upper bound.",
                        "Swap or correct the bounds.",
                    ));
                }
                Ok(Value::Float(value.clamp(minimum, maximum)))
            }
            "gcd" | "lcm" => {
                if a.len() != 2 {
                    return Err(self.arg_error(span, n, 2, a.len()));
                }
                let left = as_int(a[0].clone(), span, self)?;
                let right = as_int(a[1].clone(), span, self)?;
                integer_pair_math(n, left, right, span, self)
            }
            "factorial" => {
                one_arg(&a, span, n, self)?;
                let value = as_int(a[0].clone(), span, self)?;
                if !(0..=20).contains(&value) {
                    return Err(self.error(
                        "Value Error",
                        "factorial supports Int values from 0 through 20",
                        span,
                        "Larger factorials exceed the current Int range.",
                        "Use an Int between 0 and 20.",
                    ));
                }
                Ok(Value::Int((1..=value).product()))
            }
            "pow" => {
                if a.len() != 2 {
                    return Err(self.arg_error(span, n, 2, a.len()));
                }
                let (x, y) = numbers(a[0].clone(), a[1].clone(), span, self)?;
                let result = x.powf(y);
                if result.is_nan() {
                    Err(self.error(
                        "Math Error",
                        "pow produced an invalid result",
                        span,
                        "The base and exponent are outside the real-number domain.",
                        "Use values that produce a real result.",
                    ))
                } else {
                    Ok(Value::Float(result))
                }
            }
            "round" => {
                if !(a.len() == 1 || a.len() == 2) {
                    return Err(self.error(
                        "Argument Error",
                        "round expects one or two arguments",
                        span,
                        "Use round(value) or round(value, digits).",
                        "Pass a number and optional digit count.",
                    ));
                }
                let x = as_number(&a[0], span, self)?;
                let digits = if a.len() == 2 {
                    as_int(a[1].clone(), span, self)?
                } else {
                    0
                };
                let factor = 10f64.powi(digits.try_into().map_err(|_| {
                    self.error(
                        "Value Error",
                        "round digits are too large",
                        span,
                        "The digit count does not fit a 32-bit integer.",
                        "Use a smaller digit count.",
                    )
                })?);
                if !factor.is_finite() || factor == 0.0 {
                    return Err(self.error(
                        "Value Error",
                        "round digits are outside the supported Float range",
                        span,
                        "The decimal scale cannot be represented accurately.",
                        "Use a digit count between -308 and 308.",
                    ));
                }
                let scaled = x * factor;
                let out = if scaled.is_infinite() && x.is_finite() {
                    x
                } else {
                    scaled.round() / factor
                };
                if digits == 0 {
                    const I64_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
                    if !out.is_finite() || out < i64::MIN as f64 || out >= I64_UPPER_EXCLUSIVE {
                        return Err(self.error(
                            "Value Error",
                            "rounded value is outside Int limits",
                            span,
                            "round(value) returns Int and cannot silently clamp the result.",
                            "Use a smaller finite value or keep decimal digits.",
                        ));
                    }
                    Ok(Value::Int(out as i64))
                } else {
                    Ok(Value::Float(out))
                }
            }
            "min" | "max" => {
                if a.is_empty() {
                    return Err(self.arg_error(span, n, 1, 0));
                }
                let mut best = a[0].clone();
                for x in &a[1..] {
                    let ord = compare(x, &best)
                        .ok_or_else(|| self.type_error(span, "comparable values", x))?;
                    if (n == "min" && ord == Ordering::Less)
                        || (n == "max" && ord == Ordering::Greater)
                    {
                        best = x.clone()
                    }
                }
                Ok(best)
            }
            "sum" | "product" | "mean" | "median" | "mode" | "variance" | "std" => {
                one_arg(&a, span, n, self)?;
                let Value::List(v, _) = &a[0] else {
                    return Err(self.type_error(span, "a List", &a[0]));
                };
                aggregate(n, &v.borrow(), span, self)
            }
            _ => Err(self.unknown(n, span)),
        }
    }

    fn call_builtin_expr(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Option<Result<Value, Diagnostic>> {
        let operation = unary_math_operation(name)?;
        Some((|| {
            if args.len() != 1 {
                return Err(self.arg_error(span, name, 1, args.len()));
            }
            let value = self.eval(&args[0])?;
            let output = operation(as_number(&value, span, self)?);
            if output.is_nan() {
                Err(self.error(
                    "Math Error",
                    format!("{name} produced an invalid result"),
                    span,
                    "The input is outside the real-number domain.",
                    "Use a value in the function's valid domain.",
                ))
            } else {
                Ok(Value::Float(output))
            }
        })())
    }

    fn interpolate(&mut self, text: &str, span: Span) -> Result<Value, Diagnostic> {
        let mut out = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '{' {
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    out.push('{');
                    i += 2;
                    continue;
                }
                let start = i + 1;
                let mut depth = 1;
                i += 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1
                    } else if chars[i] == '}' {
                        depth -= 1
                    }
                    if depth > 0 {
                        i += 1
                    }
                }
                if depth != 0 {
                    return Err(self.error(
                        "String Error",
                        "unterminated interpolation",
                        span,
                        "An opening `{` has no matching `}`.",
                        "Add the closing brace or use `{{` for a literal brace.",
                    ));
                }
                let expr: String = chars[start..i].iter().collect();
                let tokens = Lexer::new(&expr, self.file).scan().map_err(|_| {
                    self.error(
                        "String Error",
                        "invalid interpolation",
                        span,
                        format!("`{expr}` is not a valid expression."),
                        "Put a valid Shine expression inside the braces.",
                    )
                })?;
                let parsed = Parser::new(tokens, &expr, self.file)
                    .expression_only()
                    .map_err(|_| {
                        self.error(
                            "String Error",
                            "invalid interpolation",
                            span,
                            format!("`{expr}` is not a valid expression."),
                            "Put a valid Shine expression inside the braces.",
                        )
                    })?;
                // Interpolation expressions are parsed into a temporary AST. Clear address-based
                // binding locations so an allocator-reused String cannot inherit an old slot.
                self.invalidate_binding_cache();
                out.push_str(&self.eval(&parsed)?.to_string());
                i += 1
            } else if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
                out.push('}');
                i += 2
            } else {
                out.push(chars[i]);
                i += 1
            }
        }
        Ok(Value::String(out))
    }

    fn freeze_value(
        &self,
        value: Value,
        span: Span,
        memo: &mut HashMap<(u8, usize), Value>,
    ) -> Result<Value, Diagnostic> {
        match value {
            Value::List(items, _) => {
                let identity = (0, Rc::as_ptr(&items) as usize);
                if let Some(frozen) = memo.get(&identity) {
                    return Ok(frozen.clone());
                }
                let target = Rc::new(RefCell::new(vec![]));
                let frozen = Value::List(target.clone(), true);
                memo.insert(identity, frozen.clone());
                let source = items.borrow().clone();
                let values = source
                    .into_iter()
                    .map(|item| self.freeze_value(item, span, memo))
                    .collect::<Result<Vec<_>, _>>()?;
                target.borrow_mut().extend(values);
                Ok(frozen)
            }
            Value::Dictionary(dictionary, _) => {
                let identity = (1, Rc::as_ptr(&dictionary) as usize);
                if let Some(frozen) = memo.get(&identity) {
                    return Ok(frozen.clone());
                }
                let (entries, key_type, value_type) = {
                    let source = dictionary.borrow();
                    (
                        source.entries.clone(),
                        source.key_type.clone(),
                        source.value_type.clone(),
                    )
                };
                let target = Rc::new(RefCell::new(DictionaryValue {
                    entries: vec![],
                    index: HashMap::default(),
                    key_type,
                    value_type,
                }));
                let frozen = Value::Dictionary(target.clone(), true);
                memo.insert(identity, frozen.clone());
                for entry in entries {
                    let scalar = dictionary_key(&entry.key, span, self)?;
                    let value = self.freeze_value(entry.value, span, memo)?;
                    target.borrow_mut().insert(scalar, entry.key, value);
                }
                Ok(frozen)
            }
            Value::Instance(instance, _) => Ok(Value::Instance(instance, true)),
            value => Ok(value),
        }
    }

    fn slice(
        &self,
        obj: Value,
        a: Option<i64>,
        z: Option<i64>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match obj {
            Value::List(v, _) => {
                let values = v.borrow();
                let (s, e) = slice_bounds(a, z, values.len(), span, self)?;
                Ok(Value::List(
                    Rc::new(RefCell::new(values[s..e].to_vec())),
                    false,
                ))
            }
            Value::String(v) => {
                let len = v.chars().count();
                let (start, end) = slice_bounds(a, z, len, span, self)?;
                let start_byte = char_byte_index(&v, start);
                let end_byte = char_byte_index(&v, end);
                Ok(Value::String(v[start_byte..end_byte].to_owned()))
            }
            v => Err(self.type_error(span, "a List or String", &v)),
        }
    }
    fn define(
        &mut self,
        n: &str,
        v: Value,
        ty: Option<TypeRef>,
        constant: bool,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if let Some(t) = &ty {
            self.ensure_type(&v, t, span)?
        }
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(n) {
            return Err(self.error(
                "Name Error",
                format!("`{n}` is already defined in this scope"),
                span,
                "Names must be unique inside one scope.",
                "Reassign it without a declaration or choose another name.",
            ));
        }
        scope.insert(
            n.into(),
            Binding {
                value: v,
                ty,
                constant,
            },
        );
        self.invalidate_binding_cache();
        Ok(())
    }
    fn reassign_in_scope(
        &mut self,
        scope: usize,
        name: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let slot = self.scopes[scope].names[name];
        self.reassign_at(scope, slot, name, value, span)
    }
    fn reassign_at(
        &mut self,
        scope: usize,
        slot: usize,
        name: &str,
        value: Value,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let (constant, ty) = {
            let binding = self.scopes[scope].binding(slot);
            (binding.constant, binding.ty.clone())
        };
        if constant {
            return Err(self.error(
                "Const Error",
                format!("cannot reassign constant `{name}`"),
                span,
                "Constants cannot be reassigned or mutated.",
                "Create a new variable instead.",
            ));
        }
        if let Some(ty) = &ty {
            self.ensure_type(&value, ty, span)?;
        }
        self.scopes[scope].binding_mut(slot).value = value;
        Ok(())
    }
    fn try_compound_numeric(
        &mut self,
        scope: usize,
        slot: usize,
        name: &str,
        operation: AssignOp,
        right: &Value,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        if self.scopes[scope].binding(slot).constant {
            return Err(self.error(
                "Const Error",
                format!("cannot reassign constant `{name}`"),
                span,
                "Constants cannot be reassigned or mutated.",
                "Create a new variable instead.",
            ));
        }
        let binding = self.scopes[scope].binding_mut(slot);
        let optimized = match (&mut binding.value, operation, right) {
            (Value::Int(left), AssignOp::Add, Value::Int(right)) => {
                *left = left
                    .checked_add(*right)
                    .ok_or_else(|| integer_overflow(span, self.file, self.source))?;
                true
            }
            (Value::Int(left), AssignOp::Subtract, Value::Int(right)) => {
                *left = left
                    .checked_sub(*right)
                    .ok_or_else(|| integer_overflow(span, self.file, self.source))?;
                true
            }
            (Value::Int(left), AssignOp::Multiply, Value::Int(right)) => {
                *left = left
                    .checked_mul(*right)
                    .ok_or_else(|| integer_overflow(span, self.file, self.source))?;
                true
            }
            (Value::Float(left), AssignOp::Add, Value::Float(right)) => {
                *left += right;
                true
            }
            (Value::Float(left), AssignOp::Subtract, Value::Float(right)) => {
                *left -= right;
                true
            }
            (Value::Float(left), AssignOp::Multiply, Value::Float(right)) => {
                *left *= right;
                true
            }
            (Value::Float(left), AssignOp::Divide, Value::Float(right)) if *right != 0.0 => {
                *left /= right;
                true
            }
            _ => false,
        };
        Ok(optimized)
    }
    fn ensure_type(&self, v: &Value, t: &TypeRef, span: Span) -> Result<(), Diagnostic> {
        self.ensure_type_inner(v, t, span, &mut StdHashSet::new())
    }

    fn ensure_type_inner(
        &self,
        v: &Value,
        t: &TypeRef,
        span: Span,
        active: &mut StdHashSet<(u8, usize)>,
    ) -> Result<(), Diagnostic> {
        let good = match t {
            TypeRef::Int => matches!(v, Value::Int(_)),
            TypeRef::Float => matches!(v, Value::Float(_)),
            TypeRef::Number => matches!(v, Value::Int(_) | Value::Float(_)),
            TypeRef::String => matches!(v, Value::String(_)),
            TypeRef::Bool => matches!(v, Value::Bool(_)),
            TypeRef::None => matches!(v, Value::None),
            TypeRef::List(item) => {
                if let Value::List(values, _) = v {
                    let identity = (0, Rc::as_ptr(values) as usize);
                    if !active.insert(identity) {
                        true
                    } else {
                        let good = item
                            .as_ref()
                            .map(|t| {
                                values
                                    .borrow()
                                    .iter()
                                    .all(|v| self.ensure_type_inner(v, t, span, active).is_ok())
                            })
                            .unwrap_or(true);
                        active.remove(&identity);
                        good
                    }
                } else {
                    false
                }
            }
            TypeRef::Dictionary(types) => {
                if let Value::Dictionary(dictionary, _) = v {
                    if let Some((key_type, value_type)) = types {
                        let identity = (1, Rc::as_ptr(dictionary) as usize);
                        if !active.insert(identity) {
                            true
                        } else {
                            let good = {
                                let dictionary = dictionary.borrow();
                                dictionary.entries.iter().all(|entry| {
                                    self.ensure_type_inner(&entry.key, key_type, span, active)
                                        .is_ok()
                                        && self
                                            .ensure_type_inner(
                                                &entry.value,
                                                value_type,
                                                span,
                                                active,
                                            )
                                            .is_ok()
                                })
                            };
                            active.remove(&identity);
                            if good {
                                let mut dictionary = dictionary.borrow_mut();
                                dictionary.key_type = Some((**key_type).clone());
                                dictionary.value_type = Some((**value_type).clone());
                            }
                            good
                        }
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
        };
        if good {
            Ok(())
        } else {
            Err(self.error(
                "Type Error",
                format!("expected {}, received {}", type_ref_name(t), v.type_name()),
                span,
                "A fixed type cannot receive this value.",
                "Use a matching value or remove the type annotation.",
            ))
        }
    }
    fn get(&self, n: &str) -> Option<Binding> {
        let frame_base = *self.frame_bases.last().unwrap_or(&2);
        self.scopes
            .iter()
            .rev()
            .enumerate()
            .find_map(|(reverse_index, scope)| {
                let index = self.scopes.len() - reverse_index - 1;
                (index <= 1 || index >= frame_base)
                    .then(|| scope.get(n).cloned())
                    .flatten()
            })
    }
    fn get_cached(&mut self, name: &str) -> Option<Value> {
        let (scope, slot) = self.binding_location_cached(name)?;
        Some(self.scopes[scope].binding(slot).value.clone())
    }
    fn get_cached_binding(&mut self, name: &str) -> Option<Binding> {
        let (scope, slot) = self.binding_location_cached(name)?;
        Some(self.scopes[scope].binding(slot).clone())
    }
    fn binding_location_cached(&mut self, name: &str) -> Option<(usize, usize)> {
        let key = name.as_ptr() as usize;
        let cache_index = ((key >> 4) ^ (key >> 17)) & (BINDING_CACHE_SIZE - 1);
        let entry = self.binding_cache[cache_index];
        if entry.generation == self.binding_cache_generation && entry.key == key {
            let (scope, slot) = match entry.location {
                CachedLocation::Absolute { scope, slot } => (scope, slot),
                CachedLocation::Relative { depth, slot } => {
                    (self.scopes.len().checked_sub(depth + 1)?, slot)
                }
                CachedLocation::Missing => return None,
            };
            return Some((scope, slot));
        }

        let Some((scope, slot)) =
            self.scopes
                .iter()
                .enumerate()
                .rev()
                .find_map(|(scope, bindings)| {
                    bindings.names.get(name).copied().map(|slot| (scope, slot))
                })
        else {
            self.binding_cache[cache_index] = BindingCacheEntry {
                key,
                generation: self.binding_cache_generation,
                location: CachedLocation::Missing,
            };
            return None;
        };
        let frame_base = *self.frame_bases.last().unwrap();
        // A function may see globals, but never the private locals of its
        // caller. Returning the caller's slot here would turn the evaluator
        // into dynamic-scope lookup and could expose data across call frames.
        if scope > 1 && scope < frame_base {
            self.binding_cache[cache_index] = BindingCacheEntry {
                key,
                generation: self.binding_cache_generation,
                location: CachedLocation::Missing,
            };
            return None;
        }
        let location = if scope <= 1 {
            Some(CachedLocation::Absolute { scope, slot })
        } else if scope >= frame_base {
            Some(CachedLocation::Relative {
                depth: self.scopes.len() - scope - 1,
                slot,
            })
        } else {
            None
        };
        if let Some(location) = location {
            self.binding_cache[cache_index] = BindingCacheEntry {
                key,
                generation: self.binding_cache_generation,
                location,
            };
        }
        Some((scope, slot))
    }
    fn invalidate_binding_cache(&mut self) {
        self.binding_cache_generation = self.binding_cache_generation.wrapping_add(1);
        if self.binding_cache_generation == 0 {
            self.binding_cache.fill(BindingCacheEntry::default());
            self.binding_cache_generation = 1;
        }
    }
    fn binding_scope(&self, name: &str) -> Option<usize> {
        let frame_base = *self.frame_bases.last().unwrap_or(&2);
        (0..self.scopes.len()).rev().find(|scope| {
            (*scope <= 1 || *scope >= frame_base) && self.scopes[*scope].contains_key(name)
        })
    }
    fn push(&mut self) {
        self.invalidate_binding_cache();
        self.scopes.push(Scope::default())
    }
    fn pop(&mut self) {
        self.invalidate_binding_cache();
        self.scopes.pop();
    }
    fn error(
        &self,
        c: impl Into<String>,
        m: impl Into<String>,
        s: Span,
        e: impl Into<String>,
        g: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::at(c, m, self.file, self.source, s, e, g)
    }
    fn unknown(&self, n: &str, s: Span) -> Diagnostic {
        self.error(
            "Name Error",
            format!("unknown name `{n}`"),
            s,
            "No variable, function, or built-in with this name is visible here.",
            "Define it before use or check the spelling.",
        )
    }
    fn type_error(&self, s: Span, expected: &str, v: &Value) -> Diagnostic {
        self.error(
            "Type Error",
            format!("expected {expected}, received {}", v.type_name()),
            s,
            "This operation does not support the supplied value.",
            "Use a value of the expected type.",
        )
    }
    fn const_error(&self, s: Span) -> Diagnostic {
        self.error(
            "Const Error",
            "cannot mutate a constant value",
            s,
            "A const protects both its binding and contained values.",
            "Call copy() first or use a non-const variable.",
        )
    }
    fn can_access(&self, class: &str) -> bool {
        self.class_stack
            .last()
            .is_some_and(|current| current == class)
    }
    fn private_error(&self, class: &str, member: &str, span: Span) -> Diagnostic {
        self.error(
            "Access Error",
            format!("`{member}` is private in class `{class}`"),
            span,
            "Private members are accessible only from methods of their class.",
            "Use a public method provided by the class.",
        )
    }
    fn index_error(&self, s: Span, i: i64, len: usize) -> Diagnostic {
        self.error(
            "Index Error",
            format!("index {i} is outside a list of length {len}"),
            s,
            "Valid indexes must refer to an existing element.",
            "Use an index between 0 and length - 1.",
        )
    }
    fn key_error(&self, span: Span, key: &Value) -> Diagnostic {
        self.error(
            "Key Error",
            format!("Dictionary has no key {}", dictionary_key_display(key)),
            span,
            "Indexing a Dictionary requires an existing key.",
            "Use have() or get() when the key may be absent.",
        )
    }
    fn zero(&self, s: Span) -> Diagnostic {
        self.error(
            "Math Error",
            "division by zero",
            s,
            "A number cannot be divided by zero.",
            "Ensure the divisor is non-zero.",
        )
    }
    fn arg_error(&self, s: Span, n: &str, want: usize, got: usize) -> Diagnostic {
        self.error(
            "Argument Error",
            format!("{n} expects {want} argument(s), received {got}"),
            s,
            "The call has the wrong number of arguments.",
            "Adjust the arguments to match the function.",
        )
    }
}

fn default_value(t: Option<&TypeRef>) -> Value {
    match t {
        Some(TypeRef::Int) => Value::Int(0),
        Some(TypeRef::Float | TypeRef::Number) => Value::Float(0.0),
        Some(TypeRef::String) => Value::String(String::new()),
        Some(TypeRef::Bool) => Value::Bool(false),
        Some(TypeRef::List(_)) => Value::List(Rc::new(RefCell::new(vec![])), false),
        Some(TypeRef::Dictionary(types)) => {
            let (key_type, value_type) = types
                .as_ref()
                .map(|(key, value)| (Some((**key).clone()), Some((**value).clone())))
                .unwrap_or((None, None));
            Value::Dictionary(
                Rc::new(RefCell::new(DictionaryValue {
                    entries: vec![],
                    index: HashMap::default(),
                    key_type,
                    value_type,
                })),
                false,
            )
        }
        _ => Value::None,
    }
}
fn type_ref_name(t: &TypeRef) -> String {
    match t {
        TypeRef::Int => "Int".into(),
        TypeRef::Float => "Float".into(),
        TypeRef::Number => "Number".into(),
        TypeRef::String => "String".into(),
        TypeRef::Bool => "Bool".into(),
        TypeRef::None => "None".into(),
        TypeRef::List(None) => "List".into(),
        TypeRef::List(Some(x)) => format!("List[{}]", type_ref_name(x)),
        TypeRef::Dictionary(None) => "Dictionary".into(),
        TypeRef::Dictionary(Some((key, value))) => format!(
            "Dictionary[{}, {}]",
            type_ref_name(key),
            type_ref_name(value)
        ),
    }
}
fn numbers(a: Value, b: Value, s: Span, e: &Evaluator) -> Result<(f64, f64), Diagnostic> {
    Ok((as_number(&a, s, e)?, as_number(&b, s, e)?))
}
fn as_number(v: &Value, s: Span, e: &Evaluator) -> Result<f64, Diagnostic> {
    match v {
        Value::Int(x) => Ok(*x as f64),
        Value::Float(x) => Ok(*x),
        v => Err(e.type_error(s, "a Number", v)),
    }
}
fn as_int(v: Value, s: Span, e: &Evaluator) -> Result<i64, Diagnostic> {
    if let Value::Int(x) = v {
        Ok(x)
    } else {
        Err(e.type_error(s, "an Int", &v))
    }
}
fn as_string<'a>(v: &'a Value, s: Span, e: &Evaluator) -> Result<&'a str, Diagnostic> {
    if let Value::String(x) = v {
        Ok(x)
    } else {
        Err(e.type_error(s, "a String", v))
    }
}
fn numeric2(
    a: Value,
    b: Value,
    s: Span,
    e: &Evaluator,
    float: impl Fn(f64, f64) -> f64,
    int: impl Fn(i64, i64) -> Option<i64>,
) -> Result<Value, Diagnostic> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => int(x, y).map(Value::Int).ok_or_else(|| {
            e.error(
                "Value Error",
                "integer overflow",
                s,
                "This result is outside Int limits.",
                "Use Float values or smaller numbers.",
            )
        }),
        (a, b) => {
            let (x, y) = numbers(a, b, s, e)?;
            Ok(Value::Float(float(x, y)))
        }
    }
}
fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.partial_cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::Int(x), Value::Float(y)) => compare_int_float_exact(*x, *y),
        (Value::Float(x), Value::Int(y)) => compare_int_float_exact(*y, *x).map(Ordering::reverse),
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

fn unique_values(values: &[Value]) -> Vec<Value> {
    let mut scalar_seen = StdHashSet::with_capacity(values.len());
    let mut complex_seen: Vec<&Value> = vec![];
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if let Some(key) = scalar_key(value) {
            if scalar_seen.insert(key) {
                unique.push(value.clone());
            }
        } else if !complex_seen.contains(&value) {
            complex_seen.push(value);
            unique.push(value.clone());
        }
    }
    unique
}

fn scalar_key(value: &Value) -> Option<ScalarKey> {
    Some(match value {
        Value::None => ScalarKey::None,
        Value::Bool(value) => ScalarKey::Bool(*value),
        Value::Int(value) => ScalarKey::Int(*value),
        Value::Float(value) => {
            if value.is_nan() {
                return None;
            }
            const I64_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
            if *value >= i64::MIN as f64 && *value < I64_UPPER_EXCLUSIVE && value.fract() == 0.0 {
                let integer = *value as i64;
                if compare_int_float_exact(integer, *value) == Some(Ordering::Equal) {
                    return Some(ScalarKey::Int(integer));
                }
            }
            ScalarKey::Float(value.to_bits())
        }
        Value::String(value) => ScalarKey::String(value.clone()),
        Value::List(..)
        | Value::Dictionary(..)
        | Value::Range(..)
        | Value::Function(..)
        | Value::Class(..)
        | Value::Instance(..) => return None,
    })
}

fn dictionary_key(
    value: &Value,
    span: Span,
    evaluator: &Evaluator,
) -> Result<ScalarKey, Diagnostic> {
    if matches!(value, Value::Float(value) if value.is_nan()) {
        return Err(evaluator.error(
            "Key Error",
            "NAN cannot be used as a Dictionary key",
            span,
            "NAN is not equal to itself and cannot identify one entry.",
            "Use a finite Float, Int, Bool, String, or none key.",
        ));
    }
    scalar_key(value).ok_or_else(|| {
        evaluator.error(
            "Type Error",
            format!("{} cannot be used as a Dictionary key", value.type_name()),
            span,
            "Dictionary keys must be scalar values.",
            "Use a String, Int, Float, Bool, or none key.",
        )
    })
}

fn dictionary_key_display(value: &Value) -> String {
    match value {
        Value::String(value) => format!("\"{value}\""),
        value => value.to_string(),
    }
}

fn compare_int_float_exact(integer: i64, float: f64) -> Option<Ordering> {
    if float.is_nan() {
        return None;
    }
    const I64_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
    if float >= I64_UPPER_EXCLUSIVE {
        return Some(Ordering::Less);
    }
    if float < i64::MIN as f64 {
        return Some(Ordering::Greater);
    }

    let truncated = float.trunc() as i64;
    match integer.cmp(&truncated) {
        Ordering::Equal if float.fract() > 0.0 => Some(Ordering::Less),
        Ordering::Equal if float.fract() < 0.0 => Some(Ordering::Greater),
        ordering => Some(ordering),
    }
}

fn numeric_numbers_equal(left: NumericNumber, right: NumericNumber) -> bool {
    match (left, right) {
        (NumericNumber::Int(left), NumericNumber::Int(right)) => left == right,
        (NumericNumber::Float(left), NumericNumber::Float(right)) => left == right,
        (NumericNumber::Int(left), NumericNumber::Float(right)) => {
            compare_int_float_exact(left, right) == Some(Ordering::Equal)
        }
        (NumericNumber::Float(left), NumericNumber::Int(right)) => {
            compare_int_float_exact(right, left) == Some(Ordering::Equal)
        }
    }
}

fn numeric_number_cmp(left: NumericNumber, right: NumericNumber) -> Option<Ordering> {
    match (left, right) {
        (NumericNumber::Int(left), NumericNumber::Int(right)) => Some(left.cmp(&right)),
        (NumericNumber::Float(left), NumericNumber::Float(right)) => left.partial_cmp(&right),
        (NumericNumber::Int(left), NumericNumber::Float(right)) => {
            compare_int_float_exact(left, right)
        }
        (NumericNumber::Float(left), NumericNumber::Int(right)) => {
            compare_int_float_exact(right, left).map(Ordering::reverse)
        }
    }
}

fn comparison_matches(ordering: Ordering, operation: VmCompareOp) -> bool {
    match operation {
        VmCompareOp::Equal => ordering == Ordering::Equal,
        VmCompareOp::NotEqual => ordering != Ordering::Equal,
        VmCompareOp::Less => ordering == Ordering::Less,
        VmCompareOp::LessEqual => ordering != Ordering::Greater,
        VmCompareOp::Greater => ordering == Ordering::Greater,
        VmCompareOp::GreaterEqual => ordering != Ordering::Less,
    }
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for value in values {
        compensated_add(&mut sum, &mut correction, value);
    }
    sum + correction
}

fn compensated_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let next = *sum + value;
    if sum.abs() >= value.abs() {
        *correction += (*sum - next) + value;
    } else {
        *correction += (value - next) + *sum;
    }
    *sum = next;
}
fn normalize_index(i: i64, len: usize) -> Option<usize> {
    let n = if i < 0 { len as i64 + i } else { i };
    if n >= 0 && (n as usize) < len {
        Some(n as usize)
    } else {
        None
    }
}

fn char_byte_index(value: &str, character_index: usize) -> usize {
    value
        .char_indices()
        .nth(character_index)
        .map_or(value.len(), |(byte, _)| byte)
}

fn list_would_cycle(target: &Rc<RefCell<Vec<Value>>>, value: &Value) -> bool {
    let target = Rc::as_ptr(target) as usize;
    let mut pending = vec![value.clone()];
    let mut seen = StdHashSet::new();
    while let Some(value) = pending.pop() {
        match value {
            Value::List(items, _) => {
                let pointer = Rc::as_ptr(&items) as usize;
                if pointer == target {
                    return true;
                }
                if seen.insert((0, pointer)) {
                    pending.extend(items.borrow().iter().cloned());
                }
            }
            Value::Instance(instance, _) => {
                let pointer = Rc::as_ptr(&instance) as usize;
                if seen.insert((1, pointer)) {
                    pending.extend(instance.borrow().fields.values().cloned());
                }
            }
            Value::Dictionary(dictionary, _) => {
                let pointer = Rc::as_ptr(&dictionary) as usize;
                if seen.insert((2, pointer)) {
                    pending.extend(
                        dictionary
                            .borrow()
                            .entries
                            .iter()
                            .map(|entry| entry.value.clone()),
                    );
                }
            }
            _ => {}
        }
    }
    false
}

fn value_reaches_instance(target: &Rc<RefCell<InstanceValue>>, value: &Value) -> bool {
    let target = Rc::as_ptr(target) as usize;
    let mut pending = vec![value.clone()];
    let mut seen = StdHashSet::new();
    while let Some(value) = pending.pop() {
        match value {
            Value::List(items, _) => {
                let pointer = Rc::as_ptr(&items) as usize;
                if seen.insert((0, pointer)) {
                    pending.extend(items.borrow().iter().cloned());
                }
            }
            Value::Instance(instance, _) => {
                let pointer = Rc::as_ptr(&instance) as usize;
                if pointer == target {
                    return true;
                }
                if seen.insert((1, pointer)) {
                    pending.extend(instance.borrow().fields.values().cloned());
                }
            }
            Value::Dictionary(dictionary, _) => {
                let pointer = Rc::as_ptr(&dictionary) as usize;
                if seen.insert((2, pointer)) {
                    pending.extend(
                        dictionary
                            .borrow()
                            .entries
                            .iter()
                            .map(|entry| entry.value.clone()),
                    );
                }
            }
            _ => {}
        }
    }
    false
}

fn slice_bounds(
    a: Option<i64>,
    z: Option<i64>,
    len: usize,
    s: Span,
    e: &Evaluator,
) -> Result<(usize, usize), Diagnostic> {
    let norm = |x: i64| {
        if x < 0 {
            (len as i64 + x).max(0) as usize
        } else {
            (x as usize).min(len)
        }
    };
    let a = a.map(norm).unwrap_or(0);
    let z = z.map(norm).unwrap_or(len);
    if a > z {
        Err(e.error(
            "Index Error",
            "slice start is after its end",
            s,
            "This MVP supports forward slices only.",
            "Use a start index less than or equal to the end.",
        ))
    } else {
        Ok((a, z))
    }
}
fn repeat_string(s: String, n: i64, span: Span, e: &Evaluator) -> Result<Value, Diagnostic> {
    if n < 0 {
        return Err(e.error(
            "Value Error",
            "repeat count cannot be negative",
            span,
            "Repetition needs a non-negative count.",
            "Use zero or a positive Int.",
        ));
    }
    let count = n as usize;
    let bytes = s.len().checked_mul(count).ok_or_else(|| {
        e.error(
            "Value Error",
            "string repetition is too large",
            span,
            "The requested result would exceed the available size range.",
            "Use a smaller repeat count or a shorter string.",
        )
    })?;
    if bytes > 256 * 1024 * 1024 {
        return Err(e.error(
            "Value Error",
            "string repetition is too large",
            span,
            "Shine limits one repeated String to 256 MiB to prevent memory exhaustion.",
            "Use a smaller repeat count or build the output incrementally.",
        ));
    }
    Ok(Value::String(s.repeat(count)))
}
fn repeat_list(
    v: Rc<RefCell<Vec<Value>>>,
    n: i64,
    span: Span,
    e: &Evaluator,
) -> Result<Value, Diagnostic> {
    if n < 0 {
        return Err(e.error(
            "Value Error",
            "repeat count cannot be negative",
            span,
            "Repetition needs a non-negative count.",
            "Use zero or a positive Int.",
        ));
    }
    let original = v.borrow();
    let count = n as usize;
    let total = original.len().checked_mul(count).ok_or_else(|| {
        e.error(
            "Value Error",
            "List repetition is too large",
            span,
            "The requested result would exceed the available size range.",
            "Use a smaller repeat count or a shorter List.",
        )
    })?;
    if total > 25_000_000 {
        return Err(e.error(
            "Value Error",
            "List repetition is too large",
            span,
            "Shine limits one repeated List to 25 million elements to prevent memory exhaustion.",
            "Use a smaller repeat count or process the data in chunks.",
        ));
    }
    let mut repeated = Vec::with_capacity(total);
    for _ in 0..count {
        repeated.extend(original.iter().cloned());
    }
    Ok(Value::List(Rc::new(RefCell::new(repeated)), false))
}
fn zero_args(a: &[Value], s: Span, n: &str, e: &Evaluator) -> Result<(), Diagnostic> {
    if a.is_empty() {
        Ok(())
    } else {
        Err(e.arg_error(s, n, 0, a.len()))
    }
}
fn one_arg(a: &[Value], s: Span, n: &str, e: &Evaluator) -> Result<(), Diagnostic> {
    if a.len() == 1 {
        Ok(())
    } else {
        Err(e.arg_error(s, n, 1, a.len()))
    }
}
fn aggregate(n: &str, v: &[Value], s: Span, e: &Evaluator) -> Result<Value, Diagnostic> {
    if v.is_empty() && !matches!(n, "sum" | "product") {
        return Err(e.error(
            "Value Error",
            format!("{n} requires a non-empty List"),
            s,
            "There is no aggregate value for an empty list.",
            "Add numeric values first.",
        ));
    }
    match n {
        "sum" => {
            if v.iter().all(|x| matches!(x, Value::Int(_))) {
                let mut total = 0i64;
                for value in v {
                    let Value::Int(value) = value else {
                        unreachable!()
                    };
                    total = total.checked_add(*value).ok_or_else(|| {
                        e.error(
                            "Value Error",
                            "integer overflow in sum",
                            s,
                            "The sum exceeds Int limits.",
                            "Use Float values or smaller numbers.",
                        )
                    })?;
                }
                Ok(Value::Int(total))
            } else {
                let mut total = 0.0;
                let mut correction = 0.0;
                for value in v {
                    compensated_add(&mut total, &mut correction, as_number(value, s, e)?);
                }
                Ok(Value::Float(total + correction))
            }
        }
        "product" => {
            if v.iter().all(|x| matches!(x, Value::Int(_))) {
                let mut total = 1i64;
                for value in v {
                    let Value::Int(value) = value else {
                        unreachable!()
                    };
                    total = total.checked_mul(*value).ok_or_else(|| {
                        e.error(
                            "Value Error",
                            "integer overflow in product",
                            s,
                            "The product exceeds Int limits.",
                            "Use Float values or smaller numbers.",
                        )
                    })?;
                }
                Ok(Value::Int(total))
            } else {
                let mut product = 1.0;
                for value in v {
                    product *= as_number(value, s, e)?;
                }
                Ok(Value::Float(product))
            }
        }
        "mean" => {
            let mut total = 0.0;
            let mut correction = 0.0;
            for value in v {
                compensated_add(&mut total, &mut correction, as_number(value, s, e)?);
            }
            Ok(Value::Float((total + correction) / v.len() as f64))
        }
        "median" => {
            let mut sorted = v
                .iter()
                .map(|value| as_number(value, s, e))
                .collect::<Result<Vec<_>, _>>()?;
            sorted.sort_by(f64::total_cmp);
            let middle = sorted.len() / 2;
            let value = if sorted.len() % 2 == 0 {
                (sorted[middle - 1] + sorted[middle]) / 2.0
            } else {
                sorted[middle]
            };
            Ok(Value::Float(value))
        }
        "variance" | "std" => {
            let mut count = 0.0;
            let mut mean = 0.0;
            let mut squared = 0.0;
            for value in v {
                let value = as_number(value, s, e)?;
                count += 1.0;
                let delta = value - mean;
                mean += delta / count;
                squared += delta * (value - mean);
            }
            let variance = squared / count;
            Ok(Value::Float(if n == "std" {
                variance.sqrt()
            } else {
                variance
            }))
        }
        "mode" => {
            let mut sorted = v
                .iter()
                .map(|value| as_number(value, s, e))
                .collect::<Result<Vec<_>, _>>()?;
            sorted.sort_by(f64::total_cmp);
            let mut best = sorted[0];
            let mut best_count = 1;
            let mut current = sorted[0];
            let mut current_count = 1;
            for value in sorted.into_iter().skip(1) {
                if value.total_cmp(&current) == Ordering::Equal {
                    current_count += 1;
                } else {
                    if current_count > best_count {
                        best = current;
                        best_count = current_count;
                    }
                    current = value;
                    current_count = 1;
                }
            }
            if current_count > best_count {
                best = current;
            }
            Ok(Value::Float(best))
        }
        "min" | "max" => {
            for value in v {
                as_number(value, s, e)?;
            }
            let mut best = &v[0];
            for value in &v[1..] {
                let ordering = compare(value, best).unwrap_or(Ordering::Equal);
                if (n == "min" && ordering == Ordering::Less)
                    || (n == "max" && ordering == Ordering::Greater)
                {
                    best = value;
                }
            }
            Ok(best.clone())
        }
        _ => unreachable!(),
    }
}
fn unary_math_operation(name: &str) -> Option<fn(f64) -> f64> {
    Some(match name {
        "abs" => f64::abs,
        "floor" => f64::floor,
        "ceil" => f64::ceil,
        "sqrt" => f64::sqrt,
        "sin" => f64::sin,
        "cos" => f64::cos,
        "tan" => f64::tan,
        "asin" => f64::asin,
        "acos" => f64::acos,
        "atan" => f64::atan,
        "log" => f64::ln,
        "log10" => f64::log10,
        "log2" => f64::log2,
        "exp" => f64::exp,
        "exp2" => f64::exp2,
        "cbrt" => f64::cbrt,
        "trunc" => f64::trunc,
        "fract" => f64::fract,
        "sinh" => f64::sinh,
        "cosh" => f64::cosh,
        "tanh" => f64::tanh,
        "asinh" => f64::asinh,
        "acosh" => f64::acosh,
        "atanh" => f64::atanh,
        "degrees" => f64::to_degrees,
        "radians" => f64::to_radians,
        _ => return None,
    })
}

fn unary_math(
    n: &str,
    a: Vec<Value>,
    s: Span,
    e: &Evaluator,
    f: impl Fn(f64) -> f64,
) -> Result<Value, Diagnostic> {
    one_arg(&a, s, n, e)?;
    let out = f(as_number(&a[0], s, e)?);
    if out.is_nan() {
        Err(e.error(
            "Math Error",
            format!("{n} produced an invalid result"),
            s,
            "The input is outside the real-number domain.",
            "Use a value in the function's valid domain.",
        ))
    } else {
        Ok(Value::Float(out))
    }
}

fn binary_math(
    n: &str,
    a: Vec<Value>,
    s: Span,
    e: &Evaluator,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Value, Diagnostic> {
    if a.len() != 2 {
        return Err(e.arg_error(s, n, 2, a.len()));
    }
    let out = f(as_number(&a[0], s, e)?, as_number(&a[1], s, e)?);
    if out.is_nan() {
        Err(e.error(
            "Math Error",
            format!("{n} produced an invalid result"),
            s,
            "The inputs are outside the real-number domain.",
            "Use values in the function's valid domain.",
        ))
    } else {
        Ok(Value::Float(out))
    }
}

fn integer_pair_math(
    n: &str,
    left: i64,
    right: i64,
    s: Span,
    e: &Evaluator,
) -> Result<Value, Diagnostic> {
    let mut a = left.unsigned_abs();
    let mut b = right.unsigned_abs();
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    let result = if n == "gcd" {
        a
    } else if left == 0 || right == 0 {
        0
    } else {
        left.unsigned_abs()
            .checked_div(a)
            .and_then(|value| value.checked_mul(right.unsigned_abs()))
            .ok_or_else(|| {
                e.error(
                    "Value Error",
                    "integer overflow in lcm",
                    s,
                    "The least common multiple exceeds Int limits.",
                    "Use smaller integers.",
                )
            })?
    };
    i64::try_from(result).map(Value::Int).map_err(|_| {
        e.error(
            "Value Error",
            format!("{n} result exceeds Int limits"),
            s,
            "The result cannot be represented as Int.",
            "Use smaller integers.",
        )
    })
}

fn integer_overflow(span: Span, file: &str, source: &str) -> Diagnostic {
    Diagnostic::at(
        "Value Error",
        "integer overflow",
        file,
        source,
        span,
        "This result is outside Int limits.",
        "Use a Float or smaller values.",
    )
}
