
#[doc = "Generated types for `http://substrait.io/schemas/simple_extensions`"]
pub mod simple_extensions {
    use serde::{Deserialize, Serialize};
    ///AggregateFunction
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "impls",
    "name"
  ],
  "properties": {
    "description": {
      "type": "string"
    },
    "impls": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "return"
        ],
        "properties": {
          "args": {
            "$ref": "#/$defs/arguments"
          },
          "decomposable": {
            "$ref": "#/$defs/decomposable"
          },
          "deterministic": {
            "$ref": "#/$defs/deterministic"
          },
          "implementation": {
            "$ref": "#/$defs/implementation"
          },
          "intermediate": {
            "$ref": "#/$defs/intermediate"
          },
          "maxset": {
            "$ref": "#/$defs/maxset"
          },
          "nullability": {
            "$ref": "#/$defs/nullabilityHandling"
          },
          "options": {
            "$ref": "#/$defs/options"
          },
          "ordered": {
            "$ref": "#/$defs/ordered"
          },
          "return": {
            "$ref": "#/$defs/returnValue"
          },
          "sessionDependent": {
            "$ref": "#/$defs/sessionDependent"
          },
          "variadic": {
            "$ref": "#/$defs/variadicBehavior"
          }
        },
        "additionalProperties": false
      },
      "minItems": 1
    },
    "name": {
      "type": "string"
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateFunction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub impls: Vec<AggregateFunctionImplsItem>,
    pub name: String,
}
impl From<&AggregateFunction> for AggregateFunction {
    fn from(value: &AggregateFunction) -> Self {
        value.clone()
    }
}
impl AggregateFunction {
    pub fn builder() -> builder::AggregateFunction {
        Default::default()
    }
}
///AggregateFunctionImplsItem
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "return"
  ],
  "properties": {
    "args": {
      "$ref": "#/$defs/arguments"
    },
    "decomposable": {
      "$ref": "#/$defs/decomposable"
    },
    "deterministic": {
      "$ref": "#/$defs/deterministic"
    },
    "implementation": {
      "$ref": "#/$defs/implementation"
    },
    "intermediate": {
      "$ref": "#/$defs/intermediate"
    },
    "maxset": {
      "$ref": "#/$defs/maxset"
    },
    "nullability": {
      "$ref": "#/$defs/nullabilityHandling"
    },
    "options": {
      "$ref": "#/$defs/options"
    },
    "ordered": {
      "$ref": "#/$defs/ordered"
    },
    "return": {
      "$ref": "#/$defs/returnValue"
    },
    "sessionDependent": {
      "$ref": "#/$defs/sessionDependent"
    },
    "variadic": {
      "$ref": "#/$defs/variadicBehavior"
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AggregateFunctionImplsItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Arguments>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decomposable: Option<Decomposable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deterministic: Option<Deterministic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<Implementation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<Intermediate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maxset: Option<Maxset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullability: Option<NullabilityHandling>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordered: Option<Ordered>,
    #[serde(rename = "return")]
    pub return_: ReturnValue,
    #[serde(
        rename = "sessionDependent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_dependent: Option<SessionDependent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variadic: Option<VariadicBehavior>,
}
impl From<&AggregateFunctionImplsItem> for AggregateFunctionImplsItem {
    fn from(value: &AggregateFunctionImplsItem) -> Self {
        value.clone()
    }
}
impl AggregateFunctionImplsItem {
    pub fn builder() -> builder::AggregateFunctionImplsItem {
        Default::default()
    }
}
///Arguments
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "array",
  "items": {
    "oneOf": [
      {
        "type": "object",
        "required": [
          "options"
        ],
        "properties": {
          "description": {
            "type": "string"
          },
          "name": {
            "type": "string"
          },
          "options": {
            "type": "array",
            "items": {
              "type": "string"
            }
          }
        },
        "additionalProperties": false
      },
      {
        "type": "object",
        "required": [
          "value"
        ],
        "properties": {
          "constant": {
            "type": "boolean"
          },
          "description": {
            "type": "string"
          },
          "name": {
            "type": "string"
          },
          "value": {
            "$ref": "#/$defs/type"
          }
        }
      },
      {
        "type": "object",
        "required": [
          "type"
        ],
        "properties": {
          "description": {
            "type": "string"
          },
          "name": {
            "type": "string"
          },
          "type": {
            "type": "string"
          }
        }
      }
    ]
  }
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Arguments(pub Vec<ArgumentsItem>);
impl std::ops::Deref for Arguments {
    type Target = Vec<ArgumentsItem>;
    fn deref(&self) -> &Vec<ArgumentsItem> {
        &self.0
    }
}
impl From<Arguments> for Vec<ArgumentsItem> {
    fn from(value: Arguments) -> Self {
        value.0
    }
}
impl From<&Arguments> for Arguments {
    fn from(value: &Arguments) -> Self {
        value.clone()
    }
}
impl From<Vec<ArgumentsItem>> for Arguments {
    fn from(value: Vec<ArgumentsItem>) -> Self {
        Self(value)
    }
}
///ArgumentsItem
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "oneOf": [
    {
      "type": "object",
      "required": [
        "options"
      ],
      "properties": {
        "description": {
          "type": "string"
        },
        "name": {
          "type": "string"
        },
        "options": {
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": [
        "value"
      ],
      "properties": {
        "constant": {
          "type": "boolean"
        },
        "description": {
          "type": "string"
        },
        "name": {
          "type": "string"
        },
        "value": {
          "$ref": "#/$defs/type"
        }
      }
    },
    {
      "type": "object",
      "required": [
        "type"
      ],
      "properties": {
        "description": {
          "type": "string"
        },
        "name": {
          "type": "string"
        },
        "type": {
          "type": "string"
        }
      }
    }
  ]
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum ArgumentsItem {
    Variant0 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        options: Vec<String>,
    },
    Variant1 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        constant: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        value: Type,
    },
    Variant2 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(rename = "type")]
        type_: String,
    },
}
impl From<&ArgumentsItem> for ArgumentsItem {
    fn from(value: &ArgumentsItem) -> Self {
        value.clone()
    }
}
///Decomposable
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "string",
  "enum": [
    "NONE",
    "ONE",
    "MANY"
  ]
}*/
/// ```
/// </details>
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize
)]
pub enum Decomposable {
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "ONE")]
    One,
    #[serde(rename = "MANY")]
    Many,
}
impl From<&Decomposable> for Decomposable {
    fn from(value: &Decomposable) -> Self {
        value.clone()
    }
}
impl ToString for Decomposable {
    fn to_string(&self) -> String {
        match *self {
            Self::None => "NONE".to_string(),
            Self::One => "ONE".to_string(),
            Self::Many => "MANY".to_string(),
        }
    }
}
impl std::str::FromStr for Decomposable {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, &'static str> {
        match value {
            "NONE" => Ok(Self::None),
            "ONE" => Ok(Self::One),
            "MANY" => Ok(Self::Many),
            _ => Err("invalid value"),
        }
    }
}
impl std::convert::TryFrom<&str> for Decomposable {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for Decomposable {
    type Error = &'static str;
    fn try_from(value: &String) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for Decomposable {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, &'static str> {
        value.parse()
    }
}
///Deterministic
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "boolean"
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Deterministic(pub bool);
impl std::ops::Deref for Deterministic {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}
impl From<Deterministic> for bool {
    fn from(value: Deterministic) -> Self {
        value.0
    }
}
impl From<&Deterministic> for Deterministic {
    fn from(value: &Deterministic) -> Self {
        value.clone()
    }
}
impl From<bool> for Deterministic {
    fn from(value: bool) -> Self {
        Self(value)
    }
}
impl std::str::FromStr for Deterministic {
    type Err = <bool as std::str::FromStr>::Err;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.parse()?))
    }
}
impl std::convert::TryFrom<&str> for Deterministic {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for Deterministic {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for Deterministic {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl ToString for Deterministic {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
///Implementation
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "additionalProperties": {
    "type": "string"
  }
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Implementation(pub std::collections::HashMap<String, String>);
impl std::ops::Deref for Implementation {
    type Target = std::collections::HashMap<String, String>;
    fn deref(&self) -> &std::collections::HashMap<String, String> {
        &self.0
    }
}
impl From<Implementation> for std::collections::HashMap<String, String> {
    fn from(value: Implementation) -> Self {
        value.0
    }
}
impl From<&Implementation> for Implementation {
    fn from(value: &Implementation) -> Self {
        value.clone()
    }
}
impl From<std::collections::HashMap<String, String>> for Implementation {
    fn from(value: std::collections::HashMap<String, String>) -> Self {
        Self(value)
    }
}
///Intermediate
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "$ref": "#/$defs/type"
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Intermediate(pub Type);
impl std::ops::Deref for Intermediate {
    type Target = Type;
    fn deref(&self) -> &Type {
        &self.0
    }
}
impl From<Intermediate> for Type {
    fn from(value: Intermediate) -> Self {
        value.0
    }
}
impl From<&Intermediate> for Intermediate {
    fn from(value: &Intermediate) -> Self {
        value.clone()
    }
}
impl From<Type> for Intermediate {
    fn from(value: Type) -> Self {
        Self(value)
    }
}
///Maxset
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "number"
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Maxset(pub f64);
impl std::ops::Deref for Maxset {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}
impl From<Maxset> for f64 {
    fn from(value: Maxset) -> Self {
        value.0
    }
}
impl From<&Maxset> for Maxset {
    fn from(value: &Maxset) -> Self {
        value.clone()
    }
}
impl From<f64> for Maxset {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
impl std::str::FromStr for Maxset {
    type Err = <f64 as std::str::FromStr>::Err;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.parse()?))
    }
}
impl std::convert::TryFrom<&str> for Maxset {
    type Error = <f64 as std::str::FromStr>::Err;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for Maxset {
    type Error = <f64 as std::str::FromStr>::Err;
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for Maxset {
    type Error = <f64 as std::str::FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl ToString for Maxset {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
///NullabilityHandling
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "string",
  "enum": [
    "MIRROR",
    "DECLARED_OUTPUT",
    "DISCRETE"
  ]
}*/
/// ```
/// </details>
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize
)]
pub enum NullabilityHandling {
    #[serde(rename = "MIRROR")]
    Mirror,
    #[serde(rename = "DECLARED_OUTPUT")]
    DeclaredOutput,
    #[serde(rename = "DISCRETE")]
    Discrete,
}
impl From<&NullabilityHandling> for NullabilityHandling {
    fn from(value: &NullabilityHandling) -> Self {
        value.clone()
    }
}
impl ToString for NullabilityHandling {
    fn to_string(&self) -> String {
        match *self {
            Self::Mirror => "MIRROR".to_string(),
            Self::DeclaredOutput => "DECLARED_OUTPUT".to_string(),
            Self::Discrete => "DISCRETE".to_string(),
        }
    }
}
impl std::str::FromStr for NullabilityHandling {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, &'static str> {
        match value {
            "MIRROR" => Ok(Self::Mirror),
            "DECLARED_OUTPUT" => Ok(Self::DeclaredOutput),
            "DISCRETE" => Ok(Self::Discrete),
            _ => Err("invalid value"),
        }
    }
}
impl std::convert::TryFrom<&str> for NullabilityHandling {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for NullabilityHandling {
    type Error = &'static str;
    fn try_from(value: &String) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for NullabilityHandling {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, &'static str> {
        value.parse()
    }
}
///Options
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "additionalProperties": {
    "type": "object",
    "required": [
      "values"
    ],
    "properties": {
      "description": {
        "type": "string"
      },
      "values": {
        "type": "array",
        "items": {
          "type": "string"
        }
      }
    },
    "additionalProperties": false
  }
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Options(pub std::collections::HashMap<String, OptionsValue>);
impl std::ops::Deref for Options {
    type Target = std::collections::HashMap<String, OptionsValue>;
    fn deref(&self) -> &std::collections::HashMap<String, OptionsValue> {
        &self.0
    }
}
impl From<Options> for std::collections::HashMap<String, OptionsValue> {
    fn from(value: Options) -> Self {
        value.0
    }
}
impl From<&Options> for Options {
    fn from(value: &Options) -> Self {
        value.clone()
    }
}
impl From<std::collections::HashMap<String, OptionsValue>> for Options {
    fn from(value: std::collections::HashMap<String, OptionsValue>) -> Self {
        Self(value)
    }
}
///OptionsValue
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "values"
  ],
  "properties": {
    "description": {
      "type": "string"
    },
    "values": {
      "type": "array",
      "items": {
        "type": "string"
      }
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionsValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub values: Vec<String>,
}
impl From<&OptionsValue> for OptionsValue {
    fn from(value: &OptionsValue) -> Self {
        value.clone()
    }
}
impl OptionsValue {
    pub fn builder() -> builder::OptionsValue {
        Default::default()
    }
}
///Ordered
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "boolean"
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ordered(pub bool);
impl std::ops::Deref for Ordered {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}
impl From<Ordered> for bool {
    fn from(value: Ordered) -> Self {
        value.0
    }
}
impl From<&Ordered> for Ordered {
    fn from(value: &Ordered) -> Self {
        value.clone()
    }
}
impl From<bool> for Ordered {
    fn from(value: bool) -> Self {
        Self(value)
    }
}
impl std::str::FromStr for Ordered {
    type Err = <bool as std::str::FromStr>::Err;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.parse()?))
    }
}
impl std::convert::TryFrom<&str> for Ordered {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for Ordered {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for Ordered {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl ToString for Ordered {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
///ReturnValue
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "$ref": "#/$defs/type"
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReturnValue(pub Type);
impl std::ops::Deref for ReturnValue {
    type Target = Type;
    fn deref(&self) -> &Type {
        &self.0
    }
}
impl From<ReturnValue> for Type {
    fn from(value: ReturnValue) -> Self {
        value.0
    }
}
impl From<&ReturnValue> for ReturnValue {
    fn from(value: &ReturnValue) -> Self {
        value.clone()
    }
}
impl From<Type> for ReturnValue {
    fn from(value: Type) -> Self {
        Self(value)
    }
}
///ScalarFunction
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "impls",
    "name"
  ],
  "properties": {
    "description": {
      "type": "string"
    },
    "impls": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "return"
        ],
        "properties": {
          "args": {
            "$ref": "#/$defs/arguments"
          },
          "deterministic": {
            "$ref": "#/$defs/deterministic"
          },
          "implementation": {
            "$ref": "#/$defs/implementation"
          },
          "nullability": {
            "$ref": "#/$defs/nullabilityHandling"
          },
          "options": {
            "$ref": "#/$defs/options"
          },
          "return": {
            "$ref": "#/$defs/returnValue"
          },
          "sessionDependent": {
            "$ref": "#/$defs/sessionDependent"
          },
          "variadic": {
            "$ref": "#/$defs/variadicBehavior"
          }
        },
        "additionalProperties": false
      },
      "minItems": 1
    },
    "name": {
      "type": "string"
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarFunction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub impls: Vec<ScalarFunctionImplsItem>,
    pub name: String,
}
impl From<&ScalarFunction> for ScalarFunction {
    fn from(value: &ScalarFunction) -> Self {
        value.clone()
    }
}
impl ScalarFunction {
    pub fn builder() -> builder::ScalarFunction {
        Default::default()
    }
}
///ScalarFunctionImplsItem
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "return"
  ],
  "properties": {
    "args": {
      "$ref": "#/$defs/arguments"
    },
    "deterministic": {
      "$ref": "#/$defs/deterministic"
    },
    "implementation": {
      "$ref": "#/$defs/implementation"
    },
    "nullability": {
      "$ref": "#/$defs/nullabilityHandling"
    },
    "options": {
      "$ref": "#/$defs/options"
    },
    "return": {
      "$ref": "#/$defs/returnValue"
    },
    "sessionDependent": {
      "$ref": "#/$defs/sessionDependent"
    },
    "variadic": {
      "$ref": "#/$defs/variadicBehavior"
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarFunctionImplsItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Arguments>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deterministic: Option<Deterministic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<Implementation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullability: Option<NullabilityHandling>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    #[serde(rename = "return")]
    pub return_: ReturnValue,
    #[serde(
        rename = "sessionDependent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_dependent: Option<SessionDependent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variadic: Option<VariadicBehavior>,
}
impl From<&ScalarFunctionImplsItem> for ScalarFunctionImplsItem {
    fn from(value: &ScalarFunctionImplsItem) -> Self {
        value.clone()
    }
}
impl ScalarFunctionImplsItem {
    pub fn builder() -> builder::ScalarFunctionImplsItem {
        Default::default()
    }
}
///SessionDependent
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "boolean"
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionDependent(pub bool);
impl std::ops::Deref for SessionDependent {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}
impl From<SessionDependent> for bool {
    fn from(value: SessionDependent) -> Self {
        value.0
    }
}
impl From<&SessionDependent> for SessionDependent {
    fn from(value: &SessionDependent) -> Self {
        value.clone()
    }
}
impl From<bool> for SessionDependent {
    fn from(value: bool) -> Self {
        Self(value)
    }
}
impl std::str::FromStr for SessionDependent {
    type Err = <bool as std::str::FromStr>::Err;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.parse()?))
    }
}
impl std::convert::TryFrom<&str> for SessionDependent {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for SessionDependent {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for SessionDependent {
    type Error = <bool as std::str::FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl ToString for SessionDependent {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
///SimpleExtensions
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "$id": "http://substrait.io/schemas/simple_extensions",
  "title": "Simple Extensions",
  "type": "object",
  "properties": {
    "aggregate_functions": {
      "type": "array",
      "items": {
        "$ref": "#/$defs/aggregateFunction"
      }
    },
    "dependencies": {
      "type": "object",
      "patternProperties": {
        "^[a-zA-Z_\\$][a-zA-Z0-9_\\$]*$": {
          "type": "string"
        }
      }
    },
    "scalar_functions": {
      "type": "array",
      "items": {
        "$ref": "#/$defs/scalarFunction"
      }
    },
    "type_variations": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "name",
          "parent"
        ],
        "properties": {
          "description": {
            "type": "string"
          },
          "functions": {
            "type": "string",
            "enum": [
              "INHERITS",
              "SEPARATE"
            ]
          },
          "name": {
            "type": "string"
          },
          "parent": {
            "$ref": "#/$defs/type"
          }
        },
        "additionalProperties": false
      },
      "minItems": 1
    },
    "types": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "name"
        ],
        "properties": {
          "name": {
            "type": "string"
          },
          "parameters": {
            "$ref": "#/$defs/type_param_defs"
          },
          "structure": {
            "$ref": "#/$defs/type"
          },
          "variadic": {
            "type": "boolean"
          }
        },
        "additionalProperties": false
      },
      "minItems": 1
    },
    "window_functions": {
      "type": "array",
      "items": {
        "$ref": "#/$defs/windowFunction"
      }
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SimpleExtensions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregate_functions: Vec<AggregateFunction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<SimpleExtensionsDependencies>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scalar_functions: Vec<ScalarFunction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_variations: Vec<SimpleExtensionsTypeVariationsItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<SimpleExtensionsTypesItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window_functions: Vec<WindowFunction>,
}
impl From<&SimpleExtensions> for SimpleExtensions {
    fn from(value: &SimpleExtensions) -> Self {
        value.clone()
    }
}
impl SimpleExtensions {
    pub fn builder() -> builder::SimpleExtensions {
        Default::default()
    }
}
///SimpleExtensionsDependencies
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "patternProperties": {
    "^[a-zA-Z_\\$][a-zA-Z0-9_\\$]*$": {
      "type": "string"
    }
  }
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SimpleExtensionsDependencies {}
impl From<&SimpleExtensionsDependencies> for SimpleExtensionsDependencies {
    fn from(value: &SimpleExtensionsDependencies) -> Self {
        value.clone()
    }
}
impl SimpleExtensionsDependencies {
    pub fn builder() -> builder::SimpleExtensionsDependencies {
        Default::default()
    }
}
///SimpleExtensionsTypeVariationsItem
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "name",
    "parent"
  ],
  "properties": {
    "description": {
      "type": "string"
    },
    "functions": {
      "type": "string",
      "enum": [
        "INHERITS",
        "SEPARATE"
      ]
    },
    "name": {
      "type": "string"
    },
    "parent": {
      "$ref": "#/$defs/type"
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SimpleExtensionsTypeVariationsItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub functions: Option<SimpleExtensionsTypeVariationsItemFunctions>,
    pub name: String,
    pub parent: Type,
}
impl From<&SimpleExtensionsTypeVariationsItem> for SimpleExtensionsTypeVariationsItem {
    fn from(value: &SimpleExtensionsTypeVariationsItem) -> Self {
        value.clone()
    }
}
impl SimpleExtensionsTypeVariationsItem {
    pub fn builder() -> builder::SimpleExtensionsTypeVariationsItem {
        Default::default()
    }
}
///SimpleExtensionsTypeVariationsItemFunctions
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "string",
  "enum": [
    "INHERITS",
    "SEPARATE"
  ]
}*/
/// ```
/// </details>
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize
)]
pub enum SimpleExtensionsTypeVariationsItemFunctions {
    #[serde(rename = "INHERITS")]
    Inherits,
    #[serde(rename = "SEPARATE")]
    Separate,
}
impl From<&SimpleExtensionsTypeVariationsItemFunctions>
for SimpleExtensionsTypeVariationsItemFunctions {
    fn from(value: &SimpleExtensionsTypeVariationsItemFunctions) -> Self {
        value.clone()
    }
}
impl ToString for SimpleExtensionsTypeVariationsItemFunctions {
    fn to_string(&self) -> String {
        match *self {
            Self::Inherits => "INHERITS".to_string(),
            Self::Separate => "SEPARATE".to_string(),
        }
    }
}
impl std::str::FromStr for SimpleExtensionsTypeVariationsItemFunctions {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, &'static str> {
        match value {
            "INHERITS" => Ok(Self::Inherits),
            "SEPARATE" => Ok(Self::Separate),
            _ => Err("invalid value"),
        }
    }
}
impl std::convert::TryFrom<&str> for SimpleExtensionsTypeVariationsItemFunctions {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for SimpleExtensionsTypeVariationsItemFunctions {
    type Error = &'static str;
    fn try_from(value: &String) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for SimpleExtensionsTypeVariationsItemFunctions {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, &'static str> {
        value.parse()
    }
}
///SimpleExtensionsTypesItem
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "name"
  ],
  "properties": {
    "name": {
      "type": "string"
    },
    "parameters": {
      "$ref": "#/$defs/type_param_defs"
    },
    "structure": {
      "$ref": "#/$defs/type"
    },
    "variadic": {
      "type": "boolean"
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SimpleExtensionsTypesItem {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<TypeParamDefs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<Type>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variadic: Option<bool>,
}
impl From<&SimpleExtensionsTypesItem> for SimpleExtensionsTypesItem {
    fn from(value: &SimpleExtensionsTypesItem) -> Self {
        value.clone()
    }
}
impl SimpleExtensionsTypesItem {
    pub fn builder() -> builder::SimpleExtensionsTypesItem {
        Default::default()
    }
}
///Type
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "oneOf": [
    {
      "type": "string"
    },
    {
      "type": "object"
    }
  ]
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Type {
    Variant0(String),
    Variant1(serde_json::Map<String, serde_json::Value>),
}
impl From<&Type> for Type {
    fn from(value: &Type) -> Self {
        value.clone()
    }
}
impl From<serde_json::Map<String, serde_json::Value>> for Type {
    fn from(value: serde_json::Map<String, serde_json::Value>) -> Self {
        Self::Variant1(value)
    }
}
///TypeParamDefs
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "array",
  "items": {
    "type": "object",
    "required": [
      "type"
    ],
    "properties": {
      "description": {
        "type": "string"
      },
      "max": {
        "type": "number"
      },
      "min": {
        "type": "number"
      },
      "name": {
        "type": "string"
      },
      "optional": {
        "type": "boolean"
      },
      "options": {
        "type": "array",
        "items": {
          "type": "string"
        },
        "minItems": 1,
        "uniqueItems": true
      },
      "type": {
        "type": "string",
        "enum": [
          "dataType",
          "boolean",
          "integer",
          "enumeration",
          "string"
        ]
      }
    }
  }
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TypeParamDefs(pub Vec<TypeParamDefsItem>);
impl std::ops::Deref for TypeParamDefs {
    type Target = Vec<TypeParamDefsItem>;
    fn deref(&self) -> &Vec<TypeParamDefsItem> {
        &self.0
    }
}
impl From<TypeParamDefs> for Vec<TypeParamDefsItem> {
    fn from(value: TypeParamDefs) -> Self {
        value.0
    }
}
impl From<&TypeParamDefs> for TypeParamDefs {
    fn from(value: &TypeParamDefs) -> Self {
        value.clone()
    }
}
impl From<Vec<TypeParamDefsItem>> for TypeParamDefs {
    fn from(value: Vec<TypeParamDefsItem>) -> Self {
        Self(value)
    }
}
///TypeParamDefsItem
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "type"
  ],
  "properties": {
    "description": {
      "type": "string"
    },
    "max": {
      "type": "number"
    },
    "min": {
      "type": "number"
    },
    "name": {
      "type": "string"
    },
    "optional": {
      "type": "boolean"
    },
    "options": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "minItems": 1,
      "uniqueItems": true
    },
    "type": {
      "type": "string",
      "enum": [
        "dataType",
        "boolean",
        "integer",
        "enumeration",
        "string"
      ]
    }
  }
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TypeParamDefsItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub type_: TypeParamDefsItemType,
}
impl From<&TypeParamDefsItem> for TypeParamDefsItem {
    fn from(value: &TypeParamDefsItem) -> Self {
        value.clone()
    }
}
impl TypeParamDefsItem {
    pub fn builder() -> builder::TypeParamDefsItem {
        Default::default()
    }
}
///TypeParamDefsItemType
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "string",
  "enum": [
    "dataType",
    "boolean",
    "integer",
    "enumeration",
    "string"
  ]
}*/
/// ```
/// </details>
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize
)]
pub enum TypeParamDefsItemType {
    #[serde(rename = "dataType")]
    DataType,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "enumeration")]
    Enumeration,
    #[serde(rename = "string")]
    String,
}
impl From<&TypeParamDefsItemType> for TypeParamDefsItemType {
    fn from(value: &TypeParamDefsItemType) -> Self {
        value.clone()
    }
}
impl ToString for TypeParamDefsItemType {
    fn to_string(&self) -> String {
        match *self {
            Self::DataType => "dataType".to_string(),
            Self::Boolean => "boolean".to_string(),
            Self::Integer => "integer".to_string(),
            Self::Enumeration => "enumeration".to_string(),
            Self::String => "string".to_string(),
        }
    }
}
impl std::str::FromStr for TypeParamDefsItemType {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, &'static str> {
        match value {
            "dataType" => Ok(Self::DataType),
            "boolean" => Ok(Self::Boolean),
            "integer" => Ok(Self::Integer),
            "enumeration" => Ok(Self::Enumeration),
            "string" => Ok(Self::String),
            _ => Err("invalid value"),
        }
    }
}
impl std::convert::TryFrom<&str> for TypeParamDefsItemType {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for TypeParamDefsItemType {
    type Error = &'static str;
    fn try_from(value: &String) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for TypeParamDefsItemType {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, &'static str> {
        value.parse()
    }
}
///VariadicBehavior
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "properties": {
    "max": {
      "type": "number"
    },
    "min": {
      "type": "number"
    },
    "parameterConsistency": {
      "type": "string",
      "enum": [
        "CONSISTENT",
        "INCONSISTENT"
      ]
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariadicBehavior {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(
        rename = "parameterConsistency",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parameter_consistency: Option<VariadicBehaviorParameterConsistency>,
}
impl From<&VariadicBehavior> for VariadicBehavior {
    fn from(value: &VariadicBehavior) -> Self {
        value.clone()
    }
}
impl VariadicBehavior {
    pub fn builder() -> builder::VariadicBehavior {
        Default::default()
    }
}
///VariadicBehaviorParameterConsistency
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "string",
  "enum": [
    "CONSISTENT",
    "INCONSISTENT"
  ]
}*/
/// ```
/// </details>
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize
)]
pub enum VariadicBehaviorParameterConsistency {
    #[serde(rename = "CONSISTENT")]
    Consistent,
    #[serde(rename = "INCONSISTENT")]
    Inconsistent,
}
impl From<&VariadicBehaviorParameterConsistency>
for VariadicBehaviorParameterConsistency {
    fn from(value: &VariadicBehaviorParameterConsistency) -> Self {
        value.clone()
    }
}
impl ToString for VariadicBehaviorParameterConsistency {
    fn to_string(&self) -> String {
        match *self {
            Self::Consistent => "CONSISTENT".to_string(),
            Self::Inconsistent => "INCONSISTENT".to_string(),
        }
    }
}
impl std::str::FromStr for VariadicBehaviorParameterConsistency {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, &'static str> {
        match value {
            "CONSISTENT" => Ok(Self::Consistent),
            "INCONSISTENT" => Ok(Self::Inconsistent),
            _ => Err("invalid value"),
        }
    }
}
impl std::convert::TryFrom<&str> for VariadicBehaviorParameterConsistency {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for VariadicBehaviorParameterConsistency {
    type Error = &'static str;
    fn try_from(value: &String) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for VariadicBehaviorParameterConsistency {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, &'static str> {
        value.parse()
    }
}
///WindowFunction
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "impls",
    "name"
  ],
  "properties": {
    "description": {
      "type": "string"
    },
    "impls": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "return"
        ],
        "properties": {
          "args": {
            "$ref": "#/$defs/arguments"
          },
          "decomposable": {
            "$ref": "#/$defs/decomposable"
          },
          "deterministic": {
            "$ref": "#/$defs/deterministic"
          },
          "implementation": {
            "$ref": "#/$defs/implementation"
          },
          "intermediate": {
            "$ref": "#/$defs/intermediate"
          },
          "maxset": {
            "$ref": "#/$defs/maxset"
          },
          "nullability": {
            "$ref": "#/$defs/nullabilityHandling"
          },
          "options": {
            "$ref": "#/$defs/options"
          },
          "ordered": {
            "$ref": "#/$defs/ordered"
          },
          "return": {
            "$ref": "#/$defs/returnValue"
          },
          "sessionDependent": {
            "$ref": "#/$defs/sessionDependent"
          },
          "variadic": {
            "$ref": "#/$defs/variadicBehavior"
          },
          "window_type": {
            "type": "string",
            "enum": [
              "STREAMING",
              "PARTITION"
            ]
          }
        },
        "additionalProperties": false
      },
      "minItems": 1
    },
    "name": {
      "type": "string"
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WindowFunction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub impls: Vec<WindowFunctionImplsItem>,
    pub name: String,
}
impl From<&WindowFunction> for WindowFunction {
    fn from(value: &WindowFunction) -> Self {
        value.clone()
    }
}
impl WindowFunction {
    pub fn builder() -> builder::WindowFunction {
        Default::default()
    }
}
///WindowFunctionImplsItem
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "object",
  "required": [
    "return"
  ],
  "properties": {
    "args": {
      "$ref": "#/$defs/arguments"
    },
    "decomposable": {
      "$ref": "#/$defs/decomposable"
    },
    "deterministic": {
      "$ref": "#/$defs/deterministic"
    },
    "implementation": {
      "$ref": "#/$defs/implementation"
    },
    "intermediate": {
      "$ref": "#/$defs/intermediate"
    },
    "maxset": {
      "$ref": "#/$defs/maxset"
    },
    "nullability": {
      "$ref": "#/$defs/nullabilityHandling"
    },
    "options": {
      "$ref": "#/$defs/options"
    },
    "ordered": {
      "$ref": "#/$defs/ordered"
    },
    "return": {
      "$ref": "#/$defs/returnValue"
    },
    "sessionDependent": {
      "$ref": "#/$defs/sessionDependent"
    },
    "variadic": {
      "$ref": "#/$defs/variadicBehavior"
    },
    "window_type": {
      "type": "string",
      "enum": [
        "STREAMING",
        "PARTITION"
      ]
    }
  },
  "additionalProperties": false
}*/
/// ```
/// </details>
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WindowFunctionImplsItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Arguments>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decomposable: Option<Decomposable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deterministic: Option<Deterministic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<Implementation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<Intermediate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maxset: Option<Maxset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nullability: Option<NullabilityHandling>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordered: Option<Ordered>,
    #[serde(rename = "return")]
    pub return_: ReturnValue,
    #[serde(
        rename = "sessionDependent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub session_dependent: Option<SessionDependent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variadic: Option<VariadicBehavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_type: Option<WindowFunctionImplsItemWindowType>,
}
impl From<&WindowFunctionImplsItem> for WindowFunctionImplsItem {
    fn from(value: &WindowFunctionImplsItem) -> Self {
        value.clone()
    }
}
impl WindowFunctionImplsItem {
    pub fn builder() -> builder::WindowFunctionImplsItem {
        Default::default()
    }
}
///WindowFunctionImplsItemWindowType
///
/// <details><summary>JSON schema</summary>
///
/// ```json
/**{
  "type": "string",
  "enum": [
    "STREAMING",
    "PARTITION"
  ]
}*/
/// ```
/// </details>
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize
)]
pub enum WindowFunctionImplsItemWindowType {
    #[serde(rename = "STREAMING")]
    Streaming,
    #[serde(rename = "PARTITION")]
    Partition,
}
impl From<&WindowFunctionImplsItemWindowType> for WindowFunctionImplsItemWindowType {
    fn from(value: &WindowFunctionImplsItemWindowType) -> Self {
        value.clone()
    }
}
impl ToString for WindowFunctionImplsItemWindowType {
    fn to_string(&self) -> String {
        match *self {
            Self::Streaming => "STREAMING".to_string(),
            Self::Partition => "PARTITION".to_string(),
        }
    }
}
impl std::str::FromStr for WindowFunctionImplsItemWindowType {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, &'static str> {
        match value {
            "STREAMING" => Ok(Self::Streaming),
            "PARTITION" => Ok(Self::Partition),
            _ => Err("invalid value"),
        }
    }
}
impl std::convert::TryFrom<&str> for WindowFunctionImplsItemWindowType {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<&String> for WindowFunctionImplsItemWindowType {
    type Error = &'static str;
    fn try_from(value: &String) -> Result<Self, &'static str> {
        value.parse()
    }
}
impl std::convert::TryFrom<String> for WindowFunctionImplsItemWindowType {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, &'static str> {
        value.parse()
    }
}
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct AggregateFunction {
        description: Result<Option<String>, String>,
        impls: Result<Vec<super::AggregateFunctionImplsItem>, String>,
        name: Result<String, String>,
    }
    impl Default for AggregateFunction {
        fn default() -> Self {
            Self {
                description: Ok(Default::default()),
                impls: Err("no value supplied for impls".to_string()),
                name: Err("no value supplied for name".to_string()),
            }
        }
    }
    impl AggregateFunction {
        pub fn description<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<String>>,
            T::Error: std::fmt::Display,
        {
            self.description = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for description: {}", e)
                });
            self
        }
        pub fn impls<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::AggregateFunctionImplsItem>>,
            T::Error: std::fmt::Display,
        {
            self.impls = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for impls: {}", e)
                });
            self
        }
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<String>,
            T::Error: std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {}", e));
            self
        }
    }
    impl std::convert::TryFrom<AggregateFunction> for super::AggregateFunction {
        type Error = String;
        fn try_from(value: AggregateFunction) -> Result<Self, String> {
            Ok(Self {
                description: value.description?,
                impls: value.impls?,
                name: value.name?,
            })
        }
    }
    impl From<super::AggregateFunction> for AggregateFunction {
        fn from(value: super::AggregateFunction) -> Self {
            Self {
                description: Ok(value.description),
                impls: Ok(value.impls),
                name: Ok(value.name),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct AggregateFunctionImplsItem {
        args: Result<Option<super::Arguments>, String>,
        decomposable: Result<Option<super::Decomposable>, String>,
        deterministic: Result<Option<super::Deterministic>, String>,
        implementation: Result<Option<super::Implementation>, String>,
        intermediate: Result<Option<super::Intermediate>, String>,
        maxset: Result<Option<super::Maxset>, String>,
        nullability: Result<Option<super::NullabilityHandling>, String>,
        options: Result<Option<super::Options>, String>,
        ordered: Result<Option<super::Ordered>, String>,
        return_: Result<super::ReturnValue, String>,
        session_dependent: Result<Option<super::SessionDependent>, String>,
        variadic: Result<Option<super::VariadicBehavior>, String>,
    }
    impl Default for AggregateFunctionImplsItem {
        fn default() -> Self {
            Self {
                args: Ok(Default::default()),
                decomposable: Ok(Default::default()),
                deterministic: Ok(Default::default()),
                implementation: Ok(Default::default()),
                intermediate: Ok(Default::default()),
                maxset: Ok(Default::default()),
                nullability: Ok(Default::default()),
                options: Ok(Default::default()),
                ordered: Ok(Default::default()),
                return_: Err("no value supplied for return_".to_string()),
                session_dependent: Ok(Default::default()),
                variadic: Ok(Default::default()),
            }
        }
    }
    impl AggregateFunctionImplsItem {
        pub fn args<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Arguments>>,
            T::Error: std::fmt::Display,
        {
            self.args = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for args: {}", e));
            self
        }
        pub fn decomposable<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Decomposable>>,
            T::Error: std::fmt::Display,
        {
            self.decomposable = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for decomposable: {}", e)
                });
            self
        }
        pub fn deterministic<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Deterministic>>,
            T::Error: std::fmt::Display,
        {
            self.deterministic = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for deterministic: {}", e)
                });
            self
        }
        pub fn implementation<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Implementation>>,
            T::Error: std::fmt::Display,
        {
            self.implementation = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for implementation: {}", e)
                });
            self
        }
        pub fn intermediate<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Intermediate>>,
            T::Error: std::fmt::Display,
        {
            self.intermediate = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for intermediate: {}", e)
                });
            self
        }
        pub fn maxset<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Maxset>>,
            T::Error: std::fmt::Display,
        {
            self.maxset = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for maxset: {}", e)
                });
            self
        }
        pub fn nullability<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::NullabilityHandling>>,
            T::Error: std::fmt::Display,
        {
            self.nullability = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for nullability: {}", e)
                });
            self
        }
        pub fn options<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Options>>,
            T::Error: std::fmt::Display,
        {
            self.options = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for options: {}", e)
                });
            self
        }
        pub fn ordered<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Ordered>>,
            T::Error: std::fmt::Display,
        {
            self.ordered = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for ordered: {}", e)
                });
            self
        }
        pub fn return_<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<super::ReturnValue>,
            T::Error: std::fmt::Display,
        {
            self.return_ = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for return_: {}", e)
                });
            self
        }
        pub fn session_dependent<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::SessionDependent>>,
            T::Error: std::fmt::Display,
        {
            self.session_dependent = value
                .try_into()
                .map_err(|e| {
                    format!(
                        "error converting supplied value for session_dependent: {}", e
                    )
                });
            self
        }
        pub fn variadic<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::VariadicBehavior>>,
            T::Error: std::fmt::Display,
        {
            self.variadic = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for variadic: {}", e)
                });
            self
        }
    }
    impl std::convert::TryFrom<AggregateFunctionImplsItem>
    for super::AggregateFunctionImplsItem {
        type Error = String;
        fn try_from(value: AggregateFunctionImplsItem) -> Result<Self, String> {
            Ok(Self {
                args: value.args?,
                decomposable: value.decomposable?,
                deterministic: value.deterministic?,
                implementation: value.implementation?,
                intermediate: value.intermediate?,
                maxset: value.maxset?,
                nullability: value.nullability?,
                options: value.options?,
                ordered: value.ordered?,
                return_: value.return_?,
                session_dependent: value.session_dependent?,
                variadic: value.variadic?,
            })
        }
    }
    impl From<super::AggregateFunctionImplsItem> for AggregateFunctionImplsItem {
        fn from(value: super::AggregateFunctionImplsItem) -> Self {
            Self {
                args: Ok(value.args),
                decomposable: Ok(value.decomposable),
                deterministic: Ok(value.deterministic),
                implementation: Ok(value.implementation),
                intermediate: Ok(value.intermediate),
                maxset: Ok(value.maxset),
                nullability: Ok(value.nullability),
                options: Ok(value.options),
                ordered: Ok(value.ordered),
                return_: Ok(value.return_),
                session_dependent: Ok(value.session_dependent),
                variadic: Ok(value.variadic),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct OptionsValue {
        description: Result<Option<String>, String>,
        values: Result<Vec<String>, String>,
    }
    impl Default for OptionsValue {
        fn default() -> Self {
            Self {
                description: Ok(Default::default()),
                values: Err("no value supplied for values".to_string()),
            }
        }
    }
    impl OptionsValue {
        pub fn description<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<String>>,
            T::Error: std::fmt::Display,
        {
            self.description = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for description: {}", e)
                });
            self
        }
        pub fn values<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<String>>,
            T::Error: std::fmt::Display,
        {
            self.values = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for values: {}", e)
                });
            self
        }
    }
    impl std::convert::TryFrom<OptionsValue> for super::OptionsValue {
        type Error = String;
        fn try_from(value: OptionsValue) -> Result<Self, String> {
            Ok(Self {
                description: value.description?,
                values: value.values?,
            })
        }
    }
    impl From<super::OptionsValue> for OptionsValue {
        fn from(value: super::OptionsValue) -> Self {
            Self {
                description: Ok(value.description),
                values: Ok(value.values),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ScalarFunction {
        description: Result<Option<String>, String>,
        impls: Result<Vec<super::ScalarFunctionImplsItem>, String>,
        name: Result<String, String>,
    }
    impl Default for ScalarFunction {
        fn default() -> Self {
            Self {
                description: Ok(Default::default()),
                impls: Err("no value supplied for impls".to_string()),
                name: Err("no value supplied for name".to_string()),
            }
        }
    }
    impl ScalarFunction {
        pub fn description<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<String>>,
            T::Error: std::fmt::Display,
        {
            self.description = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for description: {}", e)
                });
            self
        }
        pub fn impls<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::ScalarFunctionImplsItem>>,
            T::Error: std::fmt::Display,
        {
            self.impls = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for impls: {}", e)
                });
            self
        }
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<String>,
            T::Error: std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {}", e));
            self
        }
    }
    impl std::convert::TryFrom<ScalarFunction> for super::ScalarFunction {
        type Error = String;
        fn try_from(value: ScalarFunction) -> Result<Self, String> {
            Ok(Self {
                description: value.description?,
                impls: value.impls?,
                name: value.name?,
            })
        }
    }
    impl From<super::ScalarFunction> for ScalarFunction {
        fn from(value: super::ScalarFunction) -> Self {
            Self {
                description: Ok(value.description),
                impls: Ok(value.impls),
                name: Ok(value.name),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ScalarFunctionImplsItem {
        args: Result<Option<super::Arguments>, String>,
        deterministic: Result<Option<super::Deterministic>, String>,
        implementation: Result<Option<super::Implementation>, String>,
        nullability: Result<Option<super::NullabilityHandling>, String>,
        options: Result<Option<super::Options>, String>,
        return_: Result<super::ReturnValue, String>,
        session_dependent: Result<Option<super::SessionDependent>, String>,
        variadic: Result<Option<super::VariadicBehavior>, String>,
    }
    impl Default for ScalarFunctionImplsItem {
        fn default() -> Self {
            Self {
                args: Ok(Default::default()),
                deterministic: Ok(Default::default()),
                implementation: Ok(Default::default()),
                nullability: Ok(Default::default()),
                options: Ok(Default::default()),
                return_: Err("no value supplied for return_".to_string()),
                session_dependent: Ok(Default::default()),
                variadic: Ok(Default::default()),
            }
        }
    }
    impl ScalarFunctionImplsItem {
        pub fn args<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Arguments>>,
            T::Error: std::fmt::Display,
        {
            self.args = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for args: {}", e));
            self
        }
        pub fn deterministic<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Deterministic>>,
            T::Error: std::fmt::Display,
        {
            self.deterministic = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for deterministic: {}", e)
                });
            self
        }
        pub fn implementation<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Implementation>>,
            T::Error: std::fmt::Display,
        {
            self.implementation = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for implementation: {}", e)
                });
            self
        }
        pub fn nullability<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::NullabilityHandling>>,
            T::Error: std::fmt::Display,
        {
            self.nullability = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for nullability: {}", e)
                });
            self
        }
        pub fn options<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Options>>,
            T::Error: std::fmt::Display,
        {
            self.options = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for options: {}", e)
                });
            self
        }
        pub fn return_<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<super::ReturnValue>,
            T::Error: std::fmt::Display,
        {
            self.return_ = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for return_: {}", e)
                });
            self
        }
        pub fn session_dependent<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::SessionDependent>>,
            T::Error: std::fmt::Display,
        {
            self.session_dependent = value
                .try_into()
                .map_err(|e| {
                    format!(
                        "error converting supplied value for session_dependent: {}", e
                    )
                });
            self
        }
        pub fn variadic<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::VariadicBehavior>>,
            T::Error: std::fmt::Display,
        {
            self.variadic = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for variadic: {}", e)
                });
            self
        }
    }
    impl std::convert::TryFrom<ScalarFunctionImplsItem>
    for super::ScalarFunctionImplsItem {
        type Error = String;
        fn try_from(value: ScalarFunctionImplsItem) -> Result<Self, String> {
            Ok(Self {
                args: value.args?,
                deterministic: value.deterministic?,
                implementation: value.implementation?,
                nullability: value.nullability?,
                options: value.options?,
                return_: value.return_?,
                session_dependent: value.session_dependent?,
                variadic: value.variadic?,
            })
        }
    }
    impl From<super::ScalarFunctionImplsItem> for ScalarFunctionImplsItem {
        fn from(value: super::ScalarFunctionImplsItem) -> Self {
            Self {
                args: Ok(value.args),
                deterministic: Ok(value.deterministic),
                implementation: Ok(value.implementation),
                nullability: Ok(value.nullability),
                options: Ok(value.options),
                return_: Ok(value.return_),
                session_dependent: Ok(value.session_dependent),
                variadic: Ok(value.variadic),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SimpleExtensions {
        aggregate_functions: Result<Vec<super::AggregateFunction>, String>,
        dependencies: Result<Option<super::SimpleExtensionsDependencies>, String>,
        scalar_functions: Result<Vec<super::ScalarFunction>, String>,
        type_variations: Result<Vec<super::SimpleExtensionsTypeVariationsItem>, String>,
        types: Result<Vec<super::SimpleExtensionsTypesItem>, String>,
        window_functions: Result<Vec<super::WindowFunction>, String>,
    }
    impl Default for SimpleExtensions {
        fn default() -> Self {
            Self {
                aggregate_functions: Ok(Default::default()),
                dependencies: Ok(Default::default()),
                scalar_functions: Ok(Default::default()),
                type_variations: Ok(Default::default()),
                types: Ok(Default::default()),
                window_functions: Ok(Default::default()),
            }
        }
    }
    impl SimpleExtensions {
        pub fn aggregate_functions<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::AggregateFunction>>,
            T::Error: std::fmt::Display,
        {
            self.aggregate_functions = value
                .try_into()
                .map_err(|e| {
                    format!(
                        "error converting supplied value for aggregate_functions: {}", e
                    )
                });
            self
        }
        pub fn dependencies<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::SimpleExtensionsDependencies>>,
            T::Error: std::fmt::Display,
        {
            self.dependencies = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for dependencies: {}", e)
                });
            self
        }
        pub fn scalar_functions<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::ScalarFunction>>,
            T::Error: std::fmt::Display,
        {
            self.scalar_functions = value
                .try_into()
                .map_err(|e| {
                    format!(
                        "error converting supplied value for scalar_functions: {}", e
                    )
                });
            self
        }
        pub fn type_variations<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::SimpleExtensionsTypeVariationsItem>>,
            T::Error: std::fmt::Display,
        {
            self.type_variations = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for type_variations: {}", e)
                });
            self
        }
        pub fn types<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::SimpleExtensionsTypesItem>>,
            T::Error: std::fmt::Display,
        {
            self.types = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for types: {}", e)
                });
            self
        }
        pub fn window_functions<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::WindowFunction>>,
            T::Error: std::fmt::Display,
        {
            self.window_functions = value
                .try_into()
                .map_err(|e| {
                    format!(
                        "error converting supplied value for window_functions: {}", e
                    )
                });
            self
        }
    }
    impl std::convert::TryFrom<SimpleExtensions> for super::SimpleExtensions {
        type Error = String;
        fn try_from(value: SimpleExtensions) -> Result<Self, String> {
            Ok(Self {
                aggregate_functions: value.aggregate_functions?,
                dependencies: value.dependencies?,
                scalar_functions: value.scalar_functions?,
                type_variations: value.type_variations?,
                types: value.types?,
                window_functions: value.window_functions?,
            })
        }
    }
    impl From<super::SimpleExtensions> for SimpleExtensions {
        fn from(value: super::SimpleExtensions) -> Self {
            Self {
                aggregate_functions: Ok(value.aggregate_functions),
                dependencies: Ok(value.dependencies),
                scalar_functions: Ok(value.scalar_functions),
                type_variations: Ok(value.type_variations),
                types: Ok(value.types),
                window_functions: Ok(value.window_functions),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SimpleExtensionsDependencies {}
    impl Default for SimpleExtensionsDependencies {
        fn default() -> Self {
            Self {}
        }
    }
    impl SimpleExtensionsDependencies {}
    impl std::convert::TryFrom<SimpleExtensionsDependencies>
    for super::SimpleExtensionsDependencies {
        type Error = String;
        fn try_from(value: SimpleExtensionsDependencies) -> Result<Self, String> {
            Ok(Self {})
        }
    }
    impl From<super::SimpleExtensionsDependencies> for SimpleExtensionsDependencies {
        fn from(value: super::SimpleExtensionsDependencies) -> Self {
            Self {}
        }
    }
    #[derive(Clone, Debug)]
    pub struct SimpleExtensionsTypeVariationsItem {
        description: Result<Option<String>, String>,
        functions: Result<
            Option<super::SimpleExtensionsTypeVariationsItemFunctions>,
            String,
        >,
        name: Result<String, String>,
        parent: Result<super::Type, String>,
    }
    impl Default for SimpleExtensionsTypeVariationsItem {
        fn default() -> Self {
            Self {
                description: Ok(Default::default()),
                functions: Ok(Default::default()),
                name: Err("no value supplied for name".to_string()),
                parent: Err("no value supplied for parent".to_string()),
            }
        }
    }
    impl SimpleExtensionsTypeVariationsItem {
        pub fn description<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<String>>,
            T::Error: std::fmt::Display,
        {
            self.description = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for description: {}", e)
                });
            self
        }
        pub fn functions<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<
                Option<super::SimpleExtensionsTypeVariationsItemFunctions>,
            >,
            T::Error: std::fmt::Display,
        {
            self.functions = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for functions: {}", e)
                });
            self
        }
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<String>,
            T::Error: std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {}", e));
            self
        }
        pub fn parent<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<super::Type>,
            T::Error: std::fmt::Display,
        {
            self.parent = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for parent: {}", e)
                });
            self
        }
    }
    impl std::convert::TryFrom<SimpleExtensionsTypeVariationsItem>
    for super::SimpleExtensionsTypeVariationsItem {
        type Error = String;
        fn try_from(value: SimpleExtensionsTypeVariationsItem) -> Result<Self, String> {
            Ok(Self {
                description: value.description?,
                functions: value.functions?,
                name: value.name?,
                parent: value.parent?,
            })
        }
    }
    impl From<super::SimpleExtensionsTypeVariationsItem>
    for SimpleExtensionsTypeVariationsItem {
        fn from(value: super::SimpleExtensionsTypeVariationsItem) -> Self {
            Self {
                description: Ok(value.description),
                functions: Ok(value.functions),
                name: Ok(value.name),
                parent: Ok(value.parent),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SimpleExtensionsTypesItem {
        name: Result<String, String>,
        parameters: Result<Option<super::TypeParamDefs>, String>,
        structure: Result<Option<super::Type>, String>,
        variadic: Result<Option<bool>, String>,
    }
    impl Default for SimpleExtensionsTypesItem {
        fn default() -> Self {
            Self {
                name: Err("no value supplied for name".to_string()),
                parameters: Ok(Default::default()),
                structure: Ok(Default::default()),
                variadic: Ok(Default::default()),
            }
        }
    }
    impl SimpleExtensionsTypesItem {
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<String>,
            T::Error: std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {}", e));
            self
        }
        pub fn parameters<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::TypeParamDefs>>,
            T::Error: std::fmt::Display,
        {
            self.parameters = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for parameters: {}", e)
                });
            self
        }
        pub fn structure<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Type>>,
            T::Error: std::fmt::Display,
        {
            self.structure = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for structure: {}", e)
                });
            self
        }
        pub fn variadic<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<bool>>,
            T::Error: std::fmt::Display,
        {
            self.variadic = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for variadic: {}", e)
                });
            self
        }
    }
    impl std::convert::TryFrom<SimpleExtensionsTypesItem>
    for super::SimpleExtensionsTypesItem {
        type Error = String;
        fn try_from(value: SimpleExtensionsTypesItem) -> Result<Self, String> {
            Ok(Self {
                name: value.name?,
                parameters: value.parameters?,
                structure: value.structure?,
                variadic: value.variadic?,
            })
        }
    }
    impl From<super::SimpleExtensionsTypesItem> for SimpleExtensionsTypesItem {
        fn from(value: super::SimpleExtensionsTypesItem) -> Self {
            Self {
                name: Ok(value.name),
                parameters: Ok(value.parameters),
                structure: Ok(value.structure),
                variadic: Ok(value.variadic),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct TypeParamDefsItem {
        description: Result<Option<String>, String>,
        max: Result<Option<f64>, String>,
        min: Result<Option<f64>, String>,
        name: Result<Option<String>, String>,
        optional: Result<Option<bool>, String>,
        options: Result<Option<Vec<String>>, String>,
        type_: Result<super::TypeParamDefsItemType, String>,
    }
    impl Default for TypeParamDefsItem {
        fn default() -> Self {
            Self {
                description: Ok(Default::default()),
                max: Ok(Default::default()),
                min: Ok(Default::default()),
                name: Ok(Default::default()),
                optional: Ok(Default::default()),
                options: Ok(Default::default()),
                type_: Err("no value supplied for type_".to_string()),
            }
        }
    }
    impl TypeParamDefsItem {
        pub fn description<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<String>>,
            T::Error: std::fmt::Display,
        {
            self.description = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for description: {}", e)
                });
            self
        }
        pub fn max<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<f64>>,
            T::Error: std::fmt::Display,
        {
            self.max = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for max: {}", e));
            self
        }
        pub fn min<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<f64>>,
            T::Error: std::fmt::Display,
        {
            self.min = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for min: {}", e));
            self
        }
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<String>>,
            T::Error: std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {}", e));
            self
        }
        pub fn optional<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<bool>>,
            T::Error: std::fmt::Display,
        {
            self.optional = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for optional: {}", e)
                });
            self
        }
        pub fn options<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<Vec<String>>>,
            T::Error: std::fmt::Display,
        {
            self.options = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for options: {}", e)
                });
            self
        }
        pub fn type_<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<super::TypeParamDefsItemType>,
            T::Error: std::fmt::Display,
        {
            self.type_ = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for type_: {}", e)
                });
            self
        }
    }
    impl std::convert::TryFrom<TypeParamDefsItem> for super::TypeParamDefsItem {
        type Error = String;
        fn try_from(value: TypeParamDefsItem) -> Result<Self, String> {
            Ok(Self {
                description: value.description?,
                max: value.max?,
                min: value.min?,
                name: value.name?,
                optional: value.optional?,
                options: value.options?,
                type_: value.type_?,
            })
        }
    }
    impl From<super::TypeParamDefsItem> for TypeParamDefsItem {
        fn from(value: super::TypeParamDefsItem) -> Self {
            Self {
                description: Ok(value.description),
                max: Ok(value.max),
                min: Ok(value.min),
                name: Ok(value.name),
                optional: Ok(value.optional),
                options: Ok(value.options),
                type_: Ok(value.type_),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct VariadicBehavior {
        max: Result<Option<f64>, String>,
        min: Result<Option<f64>, String>,
        parameter_consistency: Result<
            Option<super::VariadicBehaviorParameterConsistency>,
            String,
        >,
    }
    impl Default for VariadicBehavior {
        fn default() -> Self {
            Self {
                max: Ok(Default::default()),
                min: Ok(Default::default()),
                parameter_consistency: Ok(Default::default()),
            }
        }
    }
    impl VariadicBehavior {
        pub fn max<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<f64>>,
            T::Error: std::fmt::Display,
        {
            self.max = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for max: {}", e));
            self
        }
        pub fn min<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<f64>>,
            T::Error: std::fmt::Display,
        {
            self.min = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for min: {}", e));
            self
        }
        pub fn parameter_consistency<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<
                Option<super::VariadicBehaviorParameterConsistency>,
            >,
            T::Error: std::fmt::Display,
        {
            self.parameter_consistency = value
                .try_into()
                .map_err(|e| {
                    format!(
                        "error converting supplied value for parameter_consistency: {}",
                        e
                    )
                });
            self
        }
    }
    impl std::convert::TryFrom<VariadicBehavior> for super::VariadicBehavior {
        type Error = String;
        fn try_from(value: VariadicBehavior) -> Result<Self, String> {
            Ok(Self {
                max: value.max?,
                min: value.min?,
                parameter_consistency: value.parameter_consistency?,
            })
        }
    }
    impl From<super::VariadicBehavior> for VariadicBehavior {
        fn from(value: super::VariadicBehavior) -> Self {
            Self {
                max: Ok(value.max),
                min: Ok(value.min),
                parameter_consistency: Ok(value.parameter_consistency),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct WindowFunction {
        description: Result<Option<String>, String>,
        impls: Result<Vec<super::WindowFunctionImplsItem>, String>,
        name: Result<String, String>,
    }
    impl Default for WindowFunction {
        fn default() -> Self {
            Self {
                description: Ok(Default::default()),
                impls: Err("no value supplied for impls".to_string()),
                name: Err("no value supplied for name".to_string()),
            }
        }
    }
    impl WindowFunction {
        pub fn description<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<String>>,
            T::Error: std::fmt::Display,
        {
            self.description = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for description: {}", e)
                });
            self
        }
        pub fn impls<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Vec<super::WindowFunctionImplsItem>>,
            T::Error: std::fmt::Display,
        {
            self.impls = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for impls: {}", e)
                });
            self
        }
        pub fn name<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<String>,
            T::Error: std::fmt::Display,
        {
            self.name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for name: {}", e));
            self
        }
    }
    impl std::convert::TryFrom<WindowFunction> for super::WindowFunction {
        type Error = String;
        fn try_from(value: WindowFunction) -> Result<Self, String> {
            Ok(Self {
                description: value.description?,
                impls: value.impls?,
                name: value.name?,
            })
        }
    }
    impl From<super::WindowFunction> for WindowFunction {
        fn from(value: super::WindowFunction) -> Self {
            Self {
                description: Ok(value.description),
                impls: Ok(value.impls),
                name: Ok(value.name),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct WindowFunctionImplsItem {
        args: Result<Option<super::Arguments>, String>,
        decomposable: Result<Option<super::Decomposable>, String>,
        deterministic: Result<Option<super::Deterministic>, String>,
        implementation: Result<Option<super::Implementation>, String>,
        intermediate: Result<Option<super::Intermediate>, String>,
        maxset: Result<Option<super::Maxset>, String>,
        nullability: Result<Option<super::NullabilityHandling>, String>,
        options: Result<Option<super::Options>, String>,
        ordered: Result<Option<super::Ordered>, String>,
        return_: Result<super::ReturnValue, String>,
        session_dependent: Result<Option<super::SessionDependent>, String>,
        variadic: Result<Option<super::VariadicBehavior>, String>,
        window_type: Result<Option<super::WindowFunctionImplsItemWindowType>, String>,
    }
    impl Default for WindowFunctionImplsItem {
        fn default() -> Self {
            Self {
                args: Ok(Default::default()),
                decomposable: Ok(Default::default()),
                deterministic: Ok(Default::default()),
                implementation: Ok(Default::default()),
                intermediate: Ok(Default::default()),
                maxset: Ok(Default::default()),
                nullability: Ok(Default::default()),
                options: Ok(Default::default()),
                ordered: Ok(Default::default()),
                return_: Err("no value supplied for return_".to_string()),
                session_dependent: Ok(Default::default()),
                variadic: Ok(Default::default()),
                window_type: Ok(Default::default()),
            }
        }
    }
    impl WindowFunctionImplsItem {
        pub fn args<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Arguments>>,
            T::Error: std::fmt::Display,
        {
            self.args = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for args: {}", e));
            self
        }
        pub fn decomposable<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Decomposable>>,
            T::Error: std::fmt::Display,
        {
            self.decomposable = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for decomposable: {}", e)
                });
            self
        }
        pub fn deterministic<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Deterministic>>,
            T::Error: std::fmt::Display,
        {
            self.deterministic = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for deterministic: {}", e)
                });
            self
        }
        pub fn implementation<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Implementation>>,
            T::Error: std::fmt::Display,
        {
            self.implementation = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for implementation: {}", e)
                });
            self
        }
        pub fn intermediate<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Intermediate>>,
            T::Error: std::fmt::Display,
        {
            self.intermediate = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for intermediate: {}", e)
                });
            self
        }
        pub fn maxset<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Maxset>>,
            T::Error: std::fmt::Display,
        {
            self.maxset = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for maxset: {}", e)
                });
            self
        }
        pub fn nullability<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::NullabilityHandling>>,
            T::Error: std::fmt::Display,
        {
            self.nullability = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for nullability: {}", e)
                });
            self
        }
        pub fn options<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Options>>,
            T::Error: std::fmt::Display,
        {
            self.options = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for options: {}", e)
                });
            self
        }
        pub fn ordered<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::Ordered>>,
            T::Error: std::fmt::Display,
        {
            self.ordered = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for ordered: {}", e)
                });
            self
        }
        pub fn return_<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<super::ReturnValue>,
            T::Error: std::fmt::Display,
        {
            self.return_ = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for return_: {}", e)
                });
            self
        }
        pub fn session_dependent<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::SessionDependent>>,
            T::Error: std::fmt::Display,
        {
            self.session_dependent = value
                .try_into()
                .map_err(|e| {
                    format!(
                        "error converting supplied value for session_dependent: {}", e
                    )
                });
            self
        }
        pub fn variadic<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::VariadicBehavior>>,
            T::Error: std::fmt::Display,
        {
            self.variadic = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for variadic: {}", e)
                });
            self
        }
        pub fn window_type<T>(mut self, value: T) -> Self
        where
            T: std::convert::TryInto<Option<super::WindowFunctionImplsItemWindowType>>,
            T::Error: std::fmt::Display,
        {
            self.window_type = value
                .try_into()
                .map_err(|e| {
                    format!("error converting supplied value for window_type: {}", e)
                });
            self
        }
    }
    impl std::convert::TryFrom<WindowFunctionImplsItem>
    for super::WindowFunctionImplsItem {
        type Error = String;
        fn try_from(value: WindowFunctionImplsItem) -> Result<Self, String> {
            Ok(Self {
                args: value.args?,
                decomposable: value.decomposable?,
                deterministic: value.deterministic?,
                implementation: value.implementation?,
                intermediate: value.intermediate?,
                maxset: value.maxset?,
                nullability: value.nullability?,
                options: value.options?,
                ordered: value.ordered?,
                return_: value.return_?,
                session_dependent: value.session_dependent?,
                variadic: value.variadic?,
                window_type: value.window_type?,
            })
        }
    }
    impl From<super::WindowFunctionImplsItem> for WindowFunctionImplsItem {
        fn from(value: super::WindowFunctionImplsItem) -> Self {
            Self {
                args: Ok(value.args),
                decomposable: Ok(value.decomposable),
                deterministic: Ok(value.deterministic),
                implementation: Ok(value.implementation),
                intermediate: Ok(value.intermediate),
                maxset: Ok(value.maxset),
                nullability: Ok(value.nullability),
                options: Ok(value.options),
                ordered: Ok(value.ordered),
                return_: Ok(value.return_),
                session_dependent: Ok(value.session_dependent),
                variadic: Ok(value.variadic),
                window_type: Ok(value.window_type),
            }
        }
    }
}

}