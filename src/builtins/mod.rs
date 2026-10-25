use crate::types::ty::Type;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Definition of an exposed VM builtin / internal function
#[derive(Debug, Clone)]
pub struct BuiltinFn {
    pub name: &'static str,
    pub param_types: Vec<Type>,
    pub is_variadic: bool,
    pub ret_type: Type,
    pub doc: &'static str,
}

pub struct Builtins {
    map: HashMap<&'static str, BuiltinFn>,
    list: Vec<BuiltinFn>,
}

impl Builtins {
    pub fn global() -> &'static Builtins {
        static INSTANCE: OnceLock<Builtins> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let mut b = Builtins {
                map: HashMap::new(),
                list: Vec::new(),
            };
            b.register_defaults();
            b
        })
    }

    fn register(&mut self, def: BuiltinFn) {
        self.map.insert(def.name, def.clone());
        self.list.push(def);
    }

    fn register_defaults(&mut self) {
        // ── I/O & Output ─────────────────────────────────────────────────────
        self.register(BuiltinFn {
            name: "println",
            param_types: vec![],
            is_variadic: true,
            ret_type: Type::Unit,
            doc: "Prints arguments formatted with spaces and a trailing newline to stdout",
        });
        self.register(BuiltinFn {
            name: "print",
            param_types: vec![],
            is_variadic: true,
            ret_type: Type::Unit,
            doc: "Prints arguments formatted with spaces without trailing newline to stdout",
        });

        // ── System & Time ────────────────────────────────────────────────────
        self.register(BuiltinFn {
            name: "time",
            param_types: vec![],
            is_variadic: false,
            ret_type: Type::Float,
            doc: "Returns current UNIX epoch timestamp in seconds as Float",
        });

        // ── Math Builtins ────────────────────────────────────────────────────
        self.register(BuiltinFn {
            name: "sqrt",
            param_types: vec![Type::Float],
            is_variadic: false,
            ret_type: Type::Float,
            doc: "Returns square root of a Float",
        });
        self.register(BuiltinFn {
            name: "sin",
            param_types: vec![Type::Float],
            is_variadic: false,
            ret_type: Type::Float,
            doc: "Returns sine of a Float",
        });
        self.register(BuiltinFn {
            name: "cos",
            param_types: vec![Type::Float],
            is_variadic: false,
            ret_type: Type::Float,
            doc: "Returns cosine of a Float",
        });
        self.register(BuiltinFn {
            name: "tan",
            param_types: vec![Type::Float],
            is_variadic: false,
            ret_type: Type::Float,
            doc: "Returns tangent of a Float",
        });
        self.register(BuiltinFn {
            name: "abs",
            param_types: vec![Type::Float],
            is_variadic: false,
            ret_type: Type::Float,
            doc: "Returns absolute value",
        });

        // ── Collections & Buffers ────────────────────────────────────────────
        self.register(BuiltinFn {
            name: "len",
            param_types: vec![],
            is_variadic: true,
            ret_type: Type::Int,
            doc: "Returns length of string, array, or buffer",
        });

        // ── IonVM TCP Networking Internals ───────────────────────────────────
        self.register(BuiltinFn {
            name: "tcp_listen",
            param_types: vec![Type::Int],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Listens on a given TCP port and returns listener socket descriptor",
        });
        self.register(BuiltinFn {
            name: "tcp_accept",
            param_types: vec![Type::Int],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Accepts an incoming TCP connection on listener descriptor",
        });
        self.register(BuiltinFn {
            name: "tcp_recv",
            param_types: vec![Type::Int, Type::Int],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Receives bytes from a TCP socket descriptor into a String",
        });
        self.register(BuiltinFn {
            name: "tcp_send",
            param_types: vec![Type::Int, Type::String],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Sends String across a TCP socket descriptor",
        });
        self.register(BuiltinFn {
            name: "tcp_close",
            param_types: vec![Type::Int],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Closes a TCP socket descriptor",
        });
        self.register(BuiltinFn {
            name: "tcp_connect",
            param_types: vec![Type::String, Type::Int],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Connects to a TCP host and port, returning client socket descriptor",
        });

        // ── String Builtins ──────────────────────────────────────────────────
        self.register(BuiltinFn {
            name: "str_split",
            param_types: vec![Type::String, Type::String],
            is_variadic: false,
            ret_type: Type::Array(Box::new(Type::String)),
            doc: "Splits a string by a delimiter into an array of strings",
        });
        self.register(BuiltinFn {
            name: "str_slice",
            param_types: vec![Type::String, Type::Int, Type::Int],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Slices a string from start to end index",
        });
        self.register(BuiltinFn {
            name: "str_trim",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Trims leading and trailing whitespace from string",
        });
        self.register(BuiltinFn {
            name: "str_trim_start",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Trims leading whitespace from string",
        });
        self.register(BuiltinFn {
            name: "str_trim_end",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Trims trailing whitespace from string",
        });
        self.register(BuiltinFn {
            name: "str_starts_with",
            param_types: vec![Type::String, Type::String],
            is_variadic: false,
            ret_type: Type::Bool,
            doc: "Returns true if string starts with given prefix",
        });
        self.register(BuiltinFn {
            name: "str_ends_with",
            param_types: vec![Type::String, Type::String],
            is_variadic: false,
            ret_type: Type::Bool,
            doc: "Returns true if string ends with given suffix",
        });
        self.register(BuiltinFn {
            name: "str_index_of",
            param_types: vec![Type::String, Type::String],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Returns first index of needle in string or -1 if not found",
        });
        self.register(BuiltinFn {
            name: "str_replace",
            param_types: vec![Type::String, Type::String, Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Replaces all occurrences of pattern in string with replacement",
        });
        self.register(BuiltinFn {
            name: "str_char_at",
            param_types: vec![Type::String, Type::Int],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Returns character at index as a 1-character String",
        });
        self.register(BuiltinFn {
            name: "str_repeat",
            param_types: vec![Type::String, Type::Int],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Repeats string N times",
        });
        self.register(BuiltinFn {
            name: "str_to_int",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::Variant {
                cases: HashMap::from([
                    ("ok".to_string(), vec![Type::Int]),
                    ("err".to_string(), vec![Type::String]),
                ]),
                row: None,
            },
            doc: "Parses integer and returns Result[Int, String]",
        });
        self.register(BuiltinFn {
            name: "str_len",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Returns character length of string",
        });
        self.register(BuiltinFn {
            name: "to_string",
            param_types: vec![],
            is_variadic: true,
            ret_type: Type::String,
            doc: "Converts any value to its string representation",
        });

        // ── JSON Builtins ────────────────────────────────────────────────────
        self.register(BuiltinFn {
            name: "json_parse",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::Var(crate::types::ty::TypeVarId(0)),
            doc: "Parses a JSON string into an Object or Array structure",
        });
        self.register(BuiltinFn {
            name: "json_stringify",
            param_types: vec![],
            is_variadic: true,
            ret_type: Type::String,
            doc: "Serializes a value into a JSON formatted string",
        });
        self.register(BuiltinFn {
            name: "json_get",
            param_types: vec![Type::String, Type::String],
            is_variadic: false,
            ret_type: Type::Variant {
                cases: HashMap::from([
                    (
                        "some".to_string(),
                        vec![Type::Var(crate::types::ty::TypeVarId(0))],
                    ),
                    ("none".to_string(), Vec::new()),
                ]),
                row: None,
            },
            doc: "Looks up a JSON path and returns Option[Value]",
        });

        // ── File System & Stdio Builtins ──────────────────────────────────────
        self.register(BuiltinFn {
            name: "read_file",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Reads entire contents of a file into a String",
        });
        self.register(BuiltinFn {
            name: "write_file",
            param_types: vec![Type::String, Type::String],
            is_variadic: false,
            ret_type: Type::Int,
            doc: "Writes content to file, returning bytes written",
        });
        self.register(BuiltinFn {
            name: "file_exists",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::Bool,
            doc: "Returns true if file exists",
        });
        self.register(BuiltinFn {
            name: "read_line",
            param_types: vec![],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Reads a single line from standard input",
        });
        self.register(BuiltinFn {
            name: "read_stdin",
            param_types: vec![Type::Int],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Reads N bytes from standard input into a String",
        });

        // ── Syntax & Tooling Builtins (std.syntax) ───────────────────────────
        self.register(BuiltinFn {
            name: "mesh_lex",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::Var(crate::types::ty::TypeVarId(0)),
            doc: "Lexes Mesh source into an array of token objects",
        });
        self.register(BuiltinFn {
            name: "mesh_layout",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Runs indentation layout preprocessor on source",
        });
        self.register(BuiltinFn {
            name: "mesh_check",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::Var(crate::types::ty::TypeVarId(0)),
            doc: "Performs syntax, semantic, and type checking, returning diagnostic errors",
        });
        self.register(BuiltinFn {
            name: "mesh_format",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Formats Mesh source code using compiler layout engine",
        });
        self.register(BuiltinFn {
            name: "mesh_parse_ast",
            param_types: vec![Type::String],
            is_variadic: false,
            ret_type: Type::String,
            doc: "Parses source code and returns CST debug representation",
        });
    }

    /// Check if a name corresponds to a registered VM builtin
    pub fn is_builtin(name: &str) -> bool {
        Self::global().map.contains_key(name)
    }

    /// Retrieve builtin metadata
    pub fn get(name: &str) -> Option<&'static BuiltinFn> {
        Self::global().map.get(name)
    }

    /// List all registered builtins
    pub fn all() -> &'static [BuiltinFn] {
        &Self::global().list
    }
}
