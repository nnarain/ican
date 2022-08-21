// Parse a CANopen EDS file
// #![deny(missing_docs)]
#![feature(result_option_inspect)]

use std::{
    collections::HashMap,
    fs,
    io,
    path::Path,
    fmt,
};

use ini::{self, Ini, Properties};
use num_traits::Num;
use lazy_static::lazy_static;
use thiserror::Error;
use regex::Regex;

// ---------------------------------------------------------------------------------------------------------------------
// Data types to represent the content of the EDS file
// ---------------------------------------------------------------------------------------------------------------------

// EDS data types
#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    Bool,
    I8,
    I16,
    I32,
    U8,
    U16,
    U32,
    F32,
    VString,
    OString,
}

#[derive(Debug, Error, PartialEq)]
pub enum DataTypeError {
    #[error("Invalid data type '{0}'")]
    Invalid(u16)
}

impl TryFrom<u16> for DataType {
    type Error = DataTypeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0001 => Ok(DataType::Bool),
            0x0002 => Ok(DataType::I8),
            0x0003 => Ok(DataType::I16),
            0x0004 => Ok(DataType::I32),
            0x0005 => Ok(DataType::U8),
            0x0006 => Ok(DataType::U16),
            0x0007 => Ok(DataType::U32),
            0x0008 => Ok(DataType::F32),
            0x0009 => Ok(DataType::VString),
            0x000A => Ok(DataType::OString),
            _ => Err(DataTypeError::Invalid(value)),
        }
    }
}

/// Object access type
#[derive(Debug, PartialEq, Clone)]
pub enum AccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Const,
    ReadWriteProcessOutput,
    ReadWriteProcessInput,
    // TODO: rww, rwr
}

/// Access type error
#[derive(Debug, Error, PartialEq)]
pub enum AccessTypeError {
    #[error("Invalid access type: '{0}'")]
    Invalid(String)
}

impl TryFrom<&str> for AccessType {
    type Error = AccessTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ro" => Ok(AccessType::ReadOnly),
            "wo" => Ok(AccessType::WriteOnly),
            "rw" => Ok(AccessType::ReadWrite),
            "const" => Ok(AccessType::Const),
            "rww" => Ok(AccessType::ReadWriteProcessOutput),
            "rwr" => Ok(AccessType::ReadWriteProcessInput),
            _ => Err(AccessTypeError::Invalid(value.to_owned())),
        }
    }
}

/// Value type
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    U8(u8),
    U16(u16),
    U32(u32),
    F32(f32),
    VString(String),
    OString(String),
}

impl ValueType {
    pub fn to_unsigned_int(&self) -> Option<usize> {
        match *self {
            ValueType::Bool(b) => Some(b as usize),
            ValueType::U8(i) => Some(i as usize),
            ValueType::U16(i) => Some(i as usize),
            ValueType::U32(i) => Some(i as usize),
            _ => None,
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ValueType::Bool(v) => write!(f, "{}", v),
            ValueType::U8(v) => write!(f, "{}", v),
            ValueType::I8(v) => write!(f, "{}", v),
            ValueType::U16(v) => write!(f, "{}", v),
            ValueType::I16(v) => write!(f, "{}", v),
            ValueType::U32(v) => write!(f, "{}", v),
            ValueType::I32(v) => write!(f, "{}", v),
            ValueType::F32(v) => write!(f, "{}", v),
            ValueType::OString(ref v) => write!(f, "{}", v),
            ValueType::VString(ref v) => write!(f, "{}", v),
        }
    }
}

/// Variable type
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable name
    pub parameter_name: String,
    /// Value data type
    pub data_type: DataType,
    /// Access type
    pub access_type: AccessType,
    /// Variable default value
    pub default_value: ValueType,
    /// Whether this variable can be PDO mapped
    pub pdo_mapping: bool,
}

/// Array type
#[derive(Debug, PartialEq, Clone)]
pub struct ArrayInfo {
    /// Array name
    pub parameter_name: String,
    /// Number of elements in the array
    pub subnumber: u8,
}

/// Record type
#[derive(Debug, PartialEq, Clone)]
pub struct RecordInfo {
    /// Record name
    pub parameter_name: String,
    /// Number of members in the record
    pub subnumber: u8,
}

/// EDS object types
#[derive(Debug, PartialEq)]
pub enum ObjectType {
    Variable,
    Array,
    Record,
}

/// Error when parsing Objects from EDS
#[derive(Debug, Error, PartialEq)]
pub enum ObjectTypeError {
    #[error("Invalid object type: {0}")]
    Invalid(u8)
}

impl TryFrom<u8> for ObjectType {
    type Error = ObjectTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x07 => Ok(ObjectType::Variable),
            0x08 => Ok(ObjectType::Array),
            0x09 => Ok(ObjectType::Record),
            _ => Err(ObjectTypeError::Invalid(value))
        }
    }
}

/// CANopen Object ID
#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
pub struct CobId(u16, u8);

impl CobId {
    pub fn into_parts(self) -> (u16, u8) {
        (self.0, self.1)
    }

    pub fn with_subindex(&self, subindex: u8) -> CobId {
        CobId(self.0, subindex)
    }
}

impl fmt::Display for CobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04X}.{:02X}", self.0, self.1)
    }
}

/// Object in the EDS
#[derive(Debug, Clone)]
pub enum Object {
    Variable(Variable),
    Array(ArrayInfo),
    Record(RecordInfo),
}

#[derive(Debug, Error)]
pub enum ObjectError {
    #[error("Failed to convert")]
    FailedToConvert,
}

impl Object {
    pub fn into_variable(self) -> Result<Variable, ObjectError> {
        if let Object::Variable(var) = self {
            Ok(var)
        }
        else {
            Err(ObjectError::FailedToConvert)
        }
    }

    pub fn into_array(self) -> Result<ArrayInfo, ObjectError> {
        if let Object::Array(arr) = self {
            Ok(arr)
        }
        else {
            Err(ObjectError::FailedToConvert)
        }
    }

    pub fn into_record(self) -> Result<RecordInfo, ObjectError> {
        if let Object::Record(rec) = self {
            Ok(rec)
        }
        else {
            Err(ObjectError::FailedToConvert)
        }
    }

    pub fn is_variable(&self) -> bool {
        matches!(*self, Object::Variable(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(*self, Object::Array(_))
    }

    pub fn is_record(&self) -> bool {
        matches!(*self, Object::Record(_))
    }

    pub fn is_metadata(&self) -> bool {
        self.is_array() || self.is_record()
    }
}

/// An Array is composed of multiple variables of the same type
#[derive(Debug)]
pub struct Array {
    pub items: Vec<Variable>,
    pub max_len: usize,
}

/// A Record is composed of multiple variables that can be of any type
pub struct Record {
    pub items: HashMap<String, Variable>,
    pub num_items: usize,
}

// impl Record {
//     pub fn into_items(self) -> Vec<Variable> {
//         self.items.into_iter()
//                   .so
//     }
// }

/// A mapped PDO item, with its data length
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MappedPdo(CobId, u8);

/// PDO Mapping
#[derive(Debug)]
pub struct PdoMapping {
    /// A maximum of 8 objects can be mapped into a single PDO
    pub slots: [Option<MappedPdo>; 8],
}

impl PdoMapping {
    pub fn from(items: Vec<MappedPdo>) -> Self {
        let mut slots: [Option<MappedPdo>; 8] = [None; 8];
        for (slot, item) in slots.iter_mut().zip(items.into_iter()) {
            *slot = Some(item);
        }

        PdoMapping { slots, }
    }
}

/// PDO Decoder
pub struct PdoDecoder {
    pub mapping: [Option<(MappedPdo, DataType)>; 8],
}

impl PdoDecoder {
    pub fn decode(&self, data: &[u8]) -> [Option<(CobId, ValueType)>; 8] {
        let mut values: [Option<(CobId, ValueType)>; 8] = Default::default();

        let mut offset: usize = 0;

        for (item, pdo) in values.iter_mut().zip(self.mapping.iter().filter(|item| item.is_some())) {
            if let Some((pdo, data_type)) = pdo {
                let start = offset;
                let end = start + pdo.1 as usize;

                offset = end;

                if let Some(value_type) = value_type_from_bytes(&data[start..end], data_type.clone()) {
                    *item = Some((pdo.0, value_type));
                }
            }
        }

        values
    }
}

/// EDS file representation
pub struct Eds {
    /// Objects in the dictionary
    objects: HashMap<CobId, Object>,
    /// Metadata objects such as Arrays and Records.
    metadata: HashMap<CobId, Object>,
}

/// Invalid object errors
#[derive(Debug, Error)]
pub enum InvalidObjectError {
    #[error("Invalid data type specified: {0}")]
    InvalidDataType(#[from] DataTypeError),
    #[error("Invalid access type specified: {0}")]
    InvalidAccessType(#[from] AccessTypeError),
    #[error("Invalid object type specified: {0}")]
    InvalidObjectType(#[from] ObjectTypeError),
}

/// Invalid string to int conversion
// pub enum StringToIntError {

// }

/// EDS file errors
#[derive(Debug, Error)]
pub enum EdsError {
    #[error("File Error: '{0}'")]
    FileError(#[from] io::Error),
    #[error("Could not parse file: {0}")]
    ConfigError(#[from] ini::ParseError),
    #[error("Failed to convert object: {0}")]
    InvalidObject(#[from] InvalidObjectError),
    #[error("Failed to convert value from string: {0}")]
    ParseIntError(#[from] core::num::ParseIntError),
    #[error("Failed to convert value from string: {0}")]
    ParseFloatError(String),
    #[error("The section did not contain parameter {0}")]
    IncorrectProperties(String),
    #[error("Object is missing value: {0}")]
    MissingProperty(String),
    #[error("The section {0} cannot be converted to a COB-ID")]
    InvalidCobIdSection(String),
}

impl Eds {
    pub fn get_tpdo1_decoder(&self) -> Option<PdoDecoder> {
        self.get_tpdo1_mapping().map(|mapping| self.get_pdo_decoder(mapping))
    }

    pub fn get_tpdo2_decoder(&self) -> Option<PdoDecoder> {
        self.get_tpdo2_mapping().map(|mapping| self.get_pdo_decoder(mapping))
    }

    pub fn get_tpdo3_decoder(&self) -> Option<PdoDecoder> {
        self.get_tpdo3_mapping().map(|mapping| self.get_pdo_decoder(mapping))
    }

    pub fn get_tpdo4_decoder(&self) -> Option<PdoDecoder> {
        self.get_tpdo4_mapping().map(|mapping| self.get_pdo_decoder(mapping))
    }

    pub fn get_pdo_decoder(&self, pdo_mapping: PdoMapping) -> PdoDecoder {
        let mut mapping: [Option<(MappedPdo, DataType)>; 8] = Default::default();

        for (pdo, mapped) in mapping.iter_mut().zip(pdo_mapping.slots.iter()) {
            let data_type = mapped.clone().and_then(|item| self.get_variable(&item.0).map(|var| var.data_type));
            if let (Some(mapped), Some(data_type)) = (mapped, data_type) {
                *pdo = Some((*mapped, data_type));
            }
        }

        PdoDecoder {
            mapping,
        }
    }

    pub fn get_tpdo1_mapping(&self) -> Option<PdoMapping> {
        self.get_pdo_mapping(CobId(0x1A00, 0x00))
    }

    pub fn get_tpdo2_mapping(&self) -> Option<PdoMapping> {
        self.get_pdo_mapping(CobId(0x1A01, 0x00))
    }

    pub fn get_tpdo3_mapping(&self) -> Option<PdoMapping> {
        self.get_pdo_mapping(CobId(0x1A02, 0x00))
    }

    pub fn get_tpdo4_mapping(&self) -> Option<PdoMapping> {
        self.get_pdo_mapping(CobId(0x1A03, 0x00))
    }

    fn get_pdo_mapping(&self, cobid: CobId) -> Option<PdoMapping> {
        // Get TPDO mapping as array
        self.get_array(&cobid)
            // Get each mapped PDO item variable
            .map(|tpdos| tpdos.items )
            .or_else(|| self.get_record(&cobid).map(|r| r.items.values().cloned().collect()))
            // Get the value of each variable
            // In the form:
            //   IIII SSLL
            // where:
            //   I - Index
            //   S - subindex
            //   L - data length
            .map(|vars| vars.iter().filter_map(|var| var.default_value.to_unsigned_int())
            // Collect as vector of integers
            .collect::<Vec<usize>>())
            // Map the integers into MappedPdo structs
            .map(|values| {
                values.iter()
                      .map(|value| {
                            let index = ((value & 0xFFFF_0000) >> 16) as u16;
                            let subindex = ((value & 0x0000_FF00) >> 8) as u8;
                            let bit_len = (value & 0x0000_00FF) as u8;

                            // println!("{:04X}.{:02X} ({})", index, subindex, data_len);

                            MappedPdo(CobId(index, subindex), bit_len / 8)
                      })
                      .collect::<Vec<MappedPdo>>()
            })
            .map(PdoMapping::from)
    }

    pub fn get_array(&self, cobid: &CobId) -> Option<Array> {
        // Check if this object is an array
        if let Some(Object::Array(array_info)) = self.metadata.get(cobid) {
            // Subindex 0 contains the number of entries
            let num = self.get_variable(&cobid.with_subindex(0)).map(|var| var.default_value.to_unsigned_int()).flatten();

            if let Some(num) = num {
                let items = (1..=num).map(|subindex| cobid.with_subindex(subindex as u8))
                                    .filter_map(|cobid| self.get_variable(&cobid))
                                    .collect::<Vec<Variable>>();

                Some(Array {items, max_len: array_info.subnumber as usize})
            }
            else {
                None
            }
        }
        else {
            None
        }
    }

    pub fn get_record(&self, cobid: &CobId) -> Option<Record> {
        self.metadata.get(cobid)
            .and_then(|_| {
                let num = self.get_variable(&cobid.with_subindex(0))
                              .map(|v| v.default_value.to_unsigned_int()).flatten();

                if let Some(num) = num {
                    let items = (1..=num).map(|subindex| cobid.with_subindex(subindex as u8))
                                         .filter_map(|cobid| self.get_variable(&cobid))
                                         .map(|var| (var.parameter_name.clone(), var))
                                         .collect::<HashMap<_, _>>();

                    Some(Record { items, num_items: num })
                }
                else {
                    None
                }
            })
    }

    pub fn get_variable(&self, cobid: &CobId) -> Option<Variable> {
        self.objects.get(cobid).map(|obj| obj.clone().into_variable().ok()).flatten()
    }

    pub fn from_str(s: &str) -> Result<Eds, EdsError> {
        let ini = Ini::load_from_str(s).map_err(|e| EdsError::ConfigError(e))?;

        let mut objects: HashMap<CobId, Object> = HashMap::default();
        let mut metadata: HashMap<CobId, Object> = HashMap::default();

        for (section, props) in ini.iter() {
            if let (Some(section), Some(object_type_str)) = (section, props.get("ObjectType")) {
                let value = eds_string_to_num::<u8>(object_type_str)?;
                let object_type = ObjectType::try_from(value).map_err(|e| InvalidObjectError::InvalidObjectType(e))?;

                let cobid = eds_section_to_cobid(section)?;

                let object = match object_type {
                    ObjectType::Variable => if let Ok(var) = variable_from_props(props) { Some(Object::Variable(var)) } else { None },
                    ObjectType::Array => Some(Object::Array(array_from_props(props)?)),
                    ObjectType::Record => Some(Object::Record(record_from_props(props)?)),
                };

                if let Some(object) = object {
                    if !object.is_metadata() {
                        objects.insert(cobid, object);
                    }
                    else {
                        metadata.insert(cobid, object);
                    }
                }
                else {
                    println!("Failed to parse object: {}", cobid);
                }
            }
        }

        Ok(Eds{
            objects,
            metadata,
        })
    }

    pub fn from<P: AsRef<Path>>(file: P) -> Result<Eds, EdsError> {
        let s = fs::read_to_string(file).map_err(|e| EdsError::FileError(e))?;

        Eds::from_str(&s)
    }

    pub fn objects(&self) -> &HashMap<CobId, Object> {
        &self.objects
    }

    pub fn metadata(&self) -> &HashMap<CobId, Object> {
        &self.metadata
    }

}

fn variable_from_props(props: &Properties) -> Result<Variable, EdsError> {
    let parameter_name = props.get("ParameterName")
                              .ok_or(EdsError::IncorrectProperties(String::from("ParameterName")))?
                              .to_owned();
    let access_type: AccessType = props.get("AccessType")
                                       .ok_or(EdsError::IncorrectProperties(String::from("AccessType")))?
                                       .try_into()
                                       .map_err(|e| InvalidObjectError::InvalidAccessType(e))?;
    let data_type: DataType = props.get("DataType")
                                   .ok_or(EdsError::IncorrectProperties(String::from("DataType")))
                                   .map(|s| eds_string_to_num::<u16>(s))??
                                   .try_into()
                                   .map_err(|e| InvalidObjectError::InvalidDataType(e))?;
    let default_value = props.get("DefaultValue").ok_or(EdsError::MissingProperty("DefaultValue".into()))?.to_owned();
    let default_value = parse_value_type(&default_value, data_type.clone())?;
    let pdo_mapping = props.get("PDOMapping")
                           .ok_or(EdsError::MissingProperty("PDOMapping".into()))
                           .map(|s| eds_string_to_num::<u8>(s))?? != 0;


    Ok(Variable{
        parameter_name,
        access_type,
        data_type,
        default_value,
        pdo_mapping,
    })
}

fn array_from_props(props: &Properties) -> Result<ArrayInfo, EdsError> {
    let parameter_name = props.get("ParameterName")
                              .ok_or(EdsError::IncorrectProperties(String::from("ParameterName")))?
                              .to_owned();
    let subnumber = props.get("SubNumber")
                         .ok_or(EdsError::MissingProperty("SubNumber".into()))
                         .map(|s| eds_string_to_num::<u8>(s))??;

    Ok(ArrayInfo{
        parameter_name,
        subnumber,
    })
}

fn record_from_props(props: &Properties) -> Result<RecordInfo, EdsError> {
    let parameter_name = props.get("ParameterName")
                              .ok_or(EdsError::IncorrectProperties(String::from("ParameterName")))?
                              .to_owned();
    let subnumber = props.get("SubNumber")
                         .ok_or(EdsError::MissingProperty("SubNumber".into()))
                         .map(|s| eds_string_to_num::<u8>(s))??;

    Ok(RecordInfo{
        parameter_name,
        subnumber,
    })
}

fn eds_section_to_cobid(section: &str) -> Result<CobId, EdsError> {
    lazy_static! {
        static ref RE_COBID: Regex = Regex::new(r"([a-zA-Z0-9]{4})(?:sub){0,1}(\d){0,1}").unwrap();
    }

    if let Some(caps) = RE_COBID.captures(section) {
        match (caps.get(1), caps.get(2)) {
            (Some(index), None) => {
                let index = u16::from_str_radix(index.as_str(), 16)?;
                Ok(CobId(index, 0x00))
            },
            (Some(index), Some(subindex)) => {
                let index = u16::from_str_radix(index.as_str(), 16)?;
                let subindex = u8::from_str_radix(subindex.as_str(), 16)?;

                Ok(CobId(index, subindex))
            },
            _ => {
                unreachable!()
            }
        }
    }
    else {
        Err(EdsError::InvalidCobIdSection(section.to_owned()))
    }
}

fn parse_value_type(source: &str, data_type: DataType) -> Result<ValueType, EdsError> {
    match data_type {
        DataType::Bool => Ok(ValueType::Bool(eds_string_to_num::<u8>(source)? != 0)),
        DataType::U8 => Ok(ValueType::U8(eds_string_to_num::<u8>(source)?)),
        DataType::U16 => Ok(ValueType::U16(eds_string_to_num::<u16>(source)?)),
        DataType::U32 => Ok(ValueType::U32(eds_string_to_num::<u32>(source)?)),
        DataType::I8 => Ok(ValueType::I8(eds_string_to_num::<i8>(source)?)),
        DataType::I16 => Ok(ValueType::I16(eds_string_to_num::<i16>(source)?)),
        DataType::I32 => Ok(ValueType::I32(eds_string_to_num::<i32>(source)?)),
        DataType::F32 => Ok(ValueType::F32(eds_string_to_num::<f32>(source).map_err(|e| EdsError::ParseFloatError(format!("{}", e)))?)),
        DataType::VString => Ok(ValueType::VString(source.to_owned())),
        DataType::OString => Ok(ValueType::VString(source.to_owned())),
    }
}

fn value_type_from_bytes(src: &[u8], data_type: DataType) -> Option<ValueType> {
    match (data_type, src.len()) {
        (DataType::Bool, 1) => Some(ValueType::Bool(src[0] != 0)),
        (DataType::U8, 1) => Some(ValueType::U8(src[0])),
        (DataType::U16, 2) => Some(ValueType::U16(u16::from_le_bytes(src.try_into().unwrap()))),
        (DataType::U32, 4) => Some(ValueType::U32(u32::from_le_bytes(src.try_into().unwrap()))),
        (DataType::I8, 1) => Some(ValueType::I8(src[0] as i8)),
        (DataType::I16, 2) => Some(ValueType::I16(i16::from_le_bytes(src.try_into().unwrap()))),
        (DataType::I32, 4) => Some(ValueType::I32(i32::from_le_bytes(src.try_into().unwrap()))),
        _ => None,
    }
}

fn eds_string_to_num<N: Num>(s: &str) -> Result<N, N::FromStrRadixErr> {
    if s.is_empty() {
        Ok(N::zero())
    }
    else {
        let (s, radix) = if s.contains("0x") {
            (s.trim_start_matches("0x"), 16)
        }
        else {
            (s, 10)
        };
    
        N::from_str_radix(s, radix)
    }
}

#[cfg(test)]
mod tests {
    use crate::value_type_from_bytes;

    use super::*;

    #[test]
    fn pdo_decode_from_record() {
        let eds = r#"
        [1A00]
        ParameterName=Transmit PDO 1 Mapping
        ObjectType=0x9
        SubNumber=9
        
        [1A00sub0]
        ParameterName=Number of Entries
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=8
        PDOMapping=0
        
        [1A00sub1]
        ParameterName=PDO 1 Mapping for a process data variable 1
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=0x60000008
        PDOMapping=0
        
        [1A00sub2]
        ParameterName=PDO 1 Mapping for a process data variable 2
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=0x60010008
        PDOMapping=0

        [6000]
        ParameterName=Foo
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=0
        PDOMapping=0

        [6001]
        ParameterName=Bar
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=0
        PDOMapping=0
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let decoder = eds.get_tpdo1_decoder().unwrap();

        // let data = ;
        let values = decoder.decode(&[0x01u8, 0x02u8]);
        let value_map = values.into_iter().filter_map(|v| v).collect::<HashMap<_, _>>();

        assert_eq!(value_map.get(&CobId(0x6000, 0x00)).unwrap(), &ValueType::U8(0x01));
        assert_eq!(value_map.get(&CobId(0x6001, 0x00)).unwrap(), &ValueType::U8(0x02));

        // assert_eq!(value_iter.next(), Some(&Some((CobId(0x6000, 0x00), ValueType::U8(0x01)))));
        // assert_eq!(value_iter.next(), Some(&Some((CobId(0x6001, 0x00), ValueType::U8(0x02)))));

    }

    #[test]
    fn pdo_decode_from_array() {
        let eds = r#"
        [1A00]
        ParameterName=Transmit PDO 1 Mapping
        ObjectType=0x8
        SubNumber=9
        
        [1A00sub0]
        ParameterName=Number of Entries
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=8
        PDOMapping=0
        
        [1A00sub1]
        ParameterName=PDO 1 Mapping for a process data variable 1
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=0x60000008
        PDOMapping=0
        
        [1A00sub2]
        ParameterName=PDO 1 Mapping for a process data variable 2
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=0x60010008
        PDOMapping=0

        [6000]
        ParameterName=Foo
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=0
        PDOMapping=0

        [6001]
        ParameterName=Bar
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=0
        PDOMapping=0
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let decoder = eds.get_tpdo1_decoder().unwrap();

        // let data = ;
        let values = decoder.decode(&[0x01u8, 0x02u8]);
        let mut value_iter = values.iter();

        assert_eq!(value_iter.next(), Some(&Some((CobId(0x6000, 0x00), ValueType::U8(0x01)))));
        assert_eq!(value_iter.next(), Some(&Some((CobId(0x6001, 0x00), ValueType::U8(0x02)))));

    }

    #[test]
    fn convert_value_type_from_bytes() {
        assert_eq!(value_type_from_bytes(&[0], DataType::Bool), Some(ValueType::Bool(false)));
        assert_eq!(value_type_from_bytes(&[1], DataType::Bool), Some(ValueType::Bool(true)));

        assert_eq!(value_type_from_bytes(&[1], DataType::U8), Some(ValueType::U8(1)));
        assert_eq!(value_type_from_bytes(&[0xFF], DataType::I8), Some(ValueType::I8(-1)));

        assert_eq!(value_type_from_bytes(&[0xAD, 0xDE], DataType::U16), Some(ValueType::U16(0xDEAD)));
        assert_eq!(value_type_from_bytes(&[0xFF, 0xFF], DataType::I16), Some(ValueType::I16(-1)));

        assert_eq!(value_type_from_bytes(&[0xEF, 0xBE, 0xAD, 0xDE], DataType::U32), Some(ValueType::U32(0xDEADBEEF)));
        assert_eq!(value_type_from_bytes(&[0xFF, 0xFF, 0xFF, 0xFF], DataType::I32), Some(ValueType::I32(-1)));
    }

    #[test]
    fn get_tpdo1_mapping() {
        let eds = r#"
        [1A00]
        ParameterName=Transmit PDO 1 Mapping
        ObjectType=0x8
        SubNumber=9
        
        [1A00sub0]
        ParameterName=Number of Entries
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=8
        PDOMapping=0
        
        [1A00sub1]
        ParameterName=PDO 1 Mapping for a process data variable 1
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=0x60000020
        PDOMapping=0
        
        [1A00sub2]
        ParameterName=PDO 1 Mapping for a process data variable 2
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=0x60000120
        PDOMapping=0
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let tpdo_mapping = eds.get_tpdo1_mapping().unwrap();

        assert_eq!(tpdo_mapping.slots[0], Some(MappedPdo(CobId(0x6000, 0x00), 0x04)));
        assert_eq!(tpdo_mapping.slots[1], Some(MappedPdo(CobId(0x6000, 0x01), 0x04)));
        assert_eq!(tpdo_mapping.slots[2], None);
    }

    #[test]
    fn parse_array() {
        let eds = r#"
        [1600]
        ParameterName=Receive PDO 1 Mapping
        ObjectType=0x8
        SubNumber=9

        [1600sub0]
        ParameterName=Number of Entries
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=2
        PDOMapping=0
        
        [1600sub1]
        ParameterName=Foo
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=1
        PDOMapping=0
        
        [1600sub2]
        ParameterName=Bar
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=2
        PDOMapping=0
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let array = eds.get_array(&CobId(0x1600, 0x00)).unwrap();

        assert_eq!(array.max_len, 9);
        assert_eq!(array.items.len(), 2);

        let var1 = array.items[0].clone();
        assert_eq!(var1.default_value.to_unsigned_int().unwrap(), 1);

        let var2 = array.items[1].clone();
        assert_eq!(var2.default_value.to_unsigned_int().unwrap(), 2);

    }

    #[test]
    fn parse_record() {
        let eds = r#"
        [1600]
        ParameterName=Receive PDO 1 Mapping
        ObjectType=0x9
        SubNumber=9

        [1600sub0]
        ParameterName=Number of Entries
        ObjectType=0x7
        DataType=0x0005
        AccessType=rw
        DefaultValue=2
        PDOMapping=0
        
        [1600sub1]
        ParameterName=Foo
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=1
        PDOMapping=0
        
        [1600sub2]
        ParameterName=Bar
        ObjectType=0x7
        DataType=0x0007
        AccessType=rw
        DefaultValue=2
        PDOMapping=0
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let record = eds.get_record(&CobId(0x1600, 0x00)).unwrap();

        let foo = record.items.get("Foo").unwrap();
        assert_eq!(foo.default_value, ValueType::U32(1));

        let bar = record.items.get("Bar").unwrap();
        assert_eq!(bar.default_value, ValueType::U32(2));
    }

    #[test]
    fn parse_record_info() {
        let eds = r#"
        [1800]
        ParameterName=Foo
        ObjectType=0x9
        SubNumber=6
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let foo = eds.metadata().get(&CobId(0x1800, 0x00)).unwrap();
        let array = foo.clone().into_record().unwrap();

        assert_eq!(array.parameter_name, String::from("Foo"));
        assert_eq!(array.subnumber, 0x06);
    }

    #[test]
    fn parse_array_info() {
        let eds = r#"
        [1600]
        ParameterName=Foo
        ObjectType=0x8
        SubNumber=5
        "#;

        let eds = Eds::from_str(eds).unwrap();
        let foo = eds.metadata().get(&CobId(0x1600, 0x00)).unwrap();
        let array = foo.clone().into_array().unwrap();

        assert_eq!(array.parameter_name, String::from("Foo"));
        assert_eq!(array.subnumber, 0x05);
    }

    #[test]
    fn parse_variable() {
        let eds = r#"
        [607C]
        ParameterName=Foo
        ObjectType=0x7
        DataType=0x0004
        AccessType=rw
        DefaultValue=0
        PDOMapping=0
        "#;

        let eds = Eds::from_str(eds).unwrap();

        let foo = eds.objects().get(&CobId(0x607C, 0x00)).unwrap();
        let var = foo.clone().into_variable().unwrap();

        assert_eq!(var.parameter_name, String::from("Foo"));
        assert_eq!(var.data_type, DataType::I32);
        assert_eq!(var.access_type, AccessType::ReadWrite);
        assert_eq!(var.pdo_mapping, false);
    }

    #[test]
    fn section_to_cobid() {
        let section = "6000";
        let cobid = eds_section_to_cobid(section).unwrap();
        assert_eq!(cobid, CobId(0x6000, 0x00))
    }

    #[test]
    fn section_to_cobid_with_subindex() {
        let section = "6000sub2";
        let cobid = eds_section_to_cobid(section).unwrap();
        assert_eq!(cobid, CobId(0x6000, 0x02))
    }

    #[test]
    fn value_type_conversion() {
        let s = "0x01";
        let value_type = parse_value_type(s, DataType::Bool).unwrap();
        assert_eq!(value_type, ValueType::Bool(true));

        let s = "0x05";
        let value_type = parse_value_type(s, DataType::U8).unwrap();
        assert_eq!(value_type, ValueType::U8(5));

        let s = "0xDEAD";
        let value_type = parse_value_type(s, DataType::U16).unwrap();
        assert_eq!(value_type, ValueType::U16(0xDEAD));

        let s = "0xDEADBEEF";
        let value_type = parse_value_type(s, DataType::U32).unwrap();
        assert_eq!(value_type, ValueType::U32(0xDEADBEEF));
    }

    #[test]
    fn parse_data_type() {
        let r = DataType::try_from(0x0001);
        assert_eq!(r, Ok(DataType::Bool));

        let r = DataType::try_from(0x0002);
        assert_eq!(r, Ok(DataType::I8));

        let r = DataType::try_from(0x0003);
        assert_eq!(r, Ok(DataType::I16));

        let r = DataType::try_from(0x0004);
        assert_eq!(r, Ok(DataType::I32));

        let r = DataType::try_from(0x0005);
        assert_eq!(r, Ok(DataType::U8));

        let r = DataType::try_from(0x0006);
        assert_eq!(r, Ok(DataType::U16));

        let r = DataType::try_from(0x0007);
        assert_eq!(r, Ok(DataType::U32));

        let r = DataType::try_from(0x0008);
        assert_eq!(r, Ok(DataType::F32));

        let r = DataType::try_from(0x0009);
        assert_eq!(r, Ok(DataType::VString));

        let r = DataType::try_from(0x000A);
        assert_eq!(r, Ok(DataType::OString));
    }

    #[test]
    fn parse_access_type() {
        let r = AccessType::try_from("ro");
        assert_eq!(r, Ok(AccessType::ReadOnly));

        let r = AccessType::try_from("wo");
        assert_eq!(r, Ok(AccessType::WriteOnly));

        let r = AccessType::try_from("rw");
        assert_eq!(r, Ok(AccessType::ReadWrite));
    }

    #[test]
    fn parse_object_type() {
        let r = ObjectType::try_from(0x07);
        assert_eq!(r, Ok(ObjectType::Variable));

        let r = ObjectType::try_from(0x08);
        assert_eq!(r, Ok(ObjectType::Array));

        let r = ObjectType::try_from(0x09);
        assert_eq!(r, Ok(ObjectType::Record));
    }

    #[test]
    fn string_to_int_hex() {
        let s = "0x0A";
        let i = eds_string_to_num::<u8>(s).unwrap();
        assert_eq!(i, 10);
    }

    #[test]
    fn string_to_int_dec() {
        let s = "7";
        let i = eds_string_to_num::<u8>(s).unwrap();
        assert_eq!(i, 7);
    }

}
