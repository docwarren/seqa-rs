pub fn map_sequence_code(code: u8) -> char {
    match code {
        0 => '=',
        1 => 'A',
        2 => 'C',
        3 => 'M',
        4 => 'G',
        5 => 'R',
        6 => 'S',
        7 => 'V',
        8 => 'T',
        9 => 'W',
        10 => 'Y',
        11 => 'H',
        12 => 'K',
        13 => 'D',
        14 => 'B',
        _ => 'N', // Default case for any unexpected value
    }
}

pub fn get_tag_ua_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i < bytes.len() {
        Ok((format!("{}", bytes[i] as char), i + 1))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_c_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i < bytes.len() {
        Ok((format!("{}", i8::from_le_bytes([bytes[i]])), i + 1))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_uc_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i < bytes.len() {
        Ok((format!("{}", bytes[i]), i + 1))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_s_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i + 2 <= bytes.len() {
        let val_str = format!("{}", i16::from_le_bytes(bytes[i..i + 2].try_into().map_err(|e| format!("Invalid i16: {}", e))?));
        Ok((val_str, i + 2))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_us_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i + 2 <= bytes.len() {
        let val_str = format!("{}", u16::from_le_bytes(bytes[i..i + 2].try_into().map_err(|e| format!("Invalid u16: {}", e))?));
        Ok((val_str, i + 2))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_i_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i + 4 <= bytes.len() {
        let val_str = format!("{}", i32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|e| format!("Invalid i32: {}", e))?));
        Ok((val_str, i + 4))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_ui_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i + 4 <= bytes.len() {
        let val_str = format!("{}", u32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|e| format!("Invalid u32: {}", e))?));
        Ok((val_str, i + 4))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_f_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    if i + 4 <= bytes.len() {
        let val_str = format!("{}", f32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|e| format!("Invalid f32: {}", e))?));
        Ok((val_str, i + 4))
    } else {
        Err("Invalid index".into())
    }
}

pub fn get_tag_z_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    let mut end = i;
    while end < bytes.len() && bytes[end] != 0 {
        end += 1;
    }
    let value = String::from_utf8_lossy(&bytes[i..end]).to_string();
    Ok((value, end + 1)) // +1 to skip the null terminator
}

pub fn get_tag_h_value(bytes: &[u8], i: usize) -> Result<(String, usize), String> {
    let mut end = i;
    while end < bytes.len() && bytes[end] != 0 {
        end += 1;
    }
    let value = String::from_utf8_lossy(&bytes[i..end]).to_string();
    Ok((value, end + 1)) // +1 to skip the null terminator
}

pub fn map_tag_type_to_result(bytes: &[u8], i: usize, value_type: u8) -> Result<(String, usize), String> {
    match value_type {
        b'A' => get_tag_ua_value(bytes, i),
        b'c' => get_tag_c_value(bytes, i),
        b'C' => get_tag_uc_value(bytes, i),
        b's' => get_tag_s_value(bytes, i),
        b'S' => get_tag_us_value(bytes, i),
        b'i' => get_tag_i_value(bytes, i),
        b'I' => get_tag_ui_value(bytes, i),
        b'f' => get_tag_f_value(bytes, i),
        b'Z' => get_tag_z_value(bytes, i),
        b'H' => get_tag_h_value(bytes, i),
        _ => Ok((String::new(), i)), // Default case for unsupported types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_sequence_code() {
        assert_eq!(map_sequence_code(0), '=');
        assert_eq!(map_sequence_code(1), 'A');
        assert_eq!(map_sequence_code(2), 'C');
        assert_eq!(map_sequence_code(4), 'G');
        assert_eq!(map_sequence_code(8), 'T');
        assert_eq!(map_sequence_code(15), 'N');
        assert_eq!(map_sequence_code(255), 'N');
    }

    #[test]
    fn test_get_tag_ua_value() {
        let bytes = vec![b'U'];
        let (val, next) = get_tag_ua_value(&bytes, 0).unwrap();
        assert_eq!(val, "U");
        assert_eq!(next, 1);
    }

    #[test]
    fn test_get_tag_ua_value_out_of_bounds() {
        let bytes: Vec<u8> = vec![];
        assert!(get_tag_ua_value(&bytes, 0).is_err());
    }

    #[test]
    fn test_get_tag_c_value() {
        let bytes = vec![0xFE]; // -2 as i8
        let (val, next) = get_tag_c_value(&bytes, 0).unwrap();
        assert_eq!(val, "-2");
        assert_eq!(next, 1);
    }

    #[test]
    fn test_get_tag_uc_value() {
        let bytes = vec![200u8];
        let (val, next) = get_tag_uc_value(&bytes, 0).unwrap();
        assert_eq!(val, "200");
        assert_eq!(next, 1);
    }

    #[test]
    fn test_get_tag_s_value() {
        let val: i16 = -1234;
        let bytes = val.to_le_bytes().to_vec();
        let (result, next) = get_tag_s_value(&bytes, 0).unwrap();
        assert_eq!(result, "-1234");
        assert_eq!(next, 2);
    }

    #[test]
    fn test_get_tag_s_value_out_of_bounds() {
        let bytes = vec![0u8];
        assert!(get_tag_s_value(&bytes, 0).is_err());
    }

    #[test]
    fn test_get_tag_us_value() {
        let val: u16 = 5000;
        let bytes = val.to_le_bytes().to_vec();
        let (result, next) = get_tag_us_value(&bytes, 0).unwrap();
        assert_eq!(result, "5000");
        assert_eq!(next, 2);
    }

    #[test]
    fn test_get_tag_i_value() {
        let val: i32 = -100000;
        let bytes = val.to_le_bytes().to_vec();
        let (result, next) = get_tag_i_value(&bytes, 0).unwrap();
        assert_eq!(result, "-100000");
        assert_eq!(next, 4);
    }

    #[test]
    fn test_get_tag_ui_value() {
        let val: u32 = 300000;
        let bytes = val.to_le_bytes().to_vec();
        let (result, next) = get_tag_ui_value(&bytes, 0).unwrap();
        assert_eq!(result, "300000");
        assert_eq!(next, 4);
    }

    #[test]
    fn test_get_tag_f_value() {
        let val: f32 = 1.5;
        let bytes = val.to_le_bytes().to_vec();
        let (result, next) = get_tag_f_value(&bytes, 0).unwrap();
        assert_eq!(result, "1.5");
        assert_eq!(next, 4);
    }

    #[test]
    fn test_get_tag_f_value_out_of_bounds() {
        let bytes = vec![0u8; 3];
        assert!(get_tag_f_value(&bytes, 0).is_err());
    }

    #[test]
    fn test_get_tag_z_value() {
        let mut bytes = b"hello".to_vec();
        bytes.push(0); // null terminator
        let (val, next) = get_tag_z_value(&bytes, 0).unwrap();
        assert_eq!(val, "hello");
        assert_eq!(next, 6);
    }

    #[test]
    fn test_get_tag_h_value() {
        let mut bytes = b"DEADBEEF".to_vec();
        bytes.push(0);
        let (val, next) = get_tag_h_value(&bytes, 0).unwrap();
        assert_eq!(val, "DEADBEEF");
        assert_eq!(next, 9);
    }

    #[test]
    fn test_get_tag_z_value_with_offset() {
        let mut bytes = vec![0xFF, 0xFF]; // padding
        bytes.extend_from_slice(b"world");
        bytes.push(0);
        let (val, next) = get_tag_z_value(&bytes, 2).unwrap();
        assert_eq!(val, "world");
        assert_eq!(next, 8);
    }

    #[test]
    fn test_map_tag_type_to_result_unsupported() {
        let bytes = vec![0u8; 4];
        let (val, next) = map_tag_type_to_result(&bytes, 0, b'?').unwrap();
        assert_eq!(val, "");
        assert_eq!(next, 0);
    }

    #[test]
    fn test_get_tag_value_z_type() {
        // tag "RG", type 'Z', value "NA12877\0"
        let mut bytes = vec![b'R', b'G', b'Z'];
        bytes.extend_from_slice(b"NA12877");
        bytes.push(0);
        let (result, next) = get_tag_value(&bytes, 0).unwrap();
        assert_eq!(result, "RG:Z:NA12877");
        assert_eq!(next, bytes.len());
    }

    #[test]
    fn test_get_tag_value_i_type() {
        // tag "NM", type 'i' (i32), value 3
        let mut bytes = vec![b'N', b'M', b'i'];
        bytes.extend_from_slice(&3i32.to_le_bytes());
        let (result, _) = get_tag_value(&bytes, 0).unwrap();
        assert_eq!(result, "NM:i:3");
    }

    #[test]
    fn test_get_tag_value_a_type() {
        // tag "XT", type 'A', value 'U'
        let bytes = vec![b'X', b'T', b'A', b'U'];
        let (result, next) = get_tag_value(&bytes, 0).unwrap();
        assert_eq!(result, "XT:A:U");
        assert_eq!(next, 4);
    }

    #[test]
    fn test_get_tag_value_b_array_type() {
        // tag "BC", type 'B', sub-type 'C' (u8), count=3, values [10, 20, 30]
        let mut bytes = vec![b'B', b'C', b'B', b'C'];
        bytes.extend_from_slice(&3i32.to_le_bytes());
        bytes.extend_from_slice(&[10, 20, 30]);
        let (result, next) = get_tag_value(&bytes, 0).unwrap();
        assert_eq!(result, "BC:B:10,20,30");
        assert_eq!(next, bytes.len());
    }

    #[test]
    fn test_get_tag_value_numeric_types_format_as_i() {
        // 'C' (u8) should format type char as 'i'
        let mut bytes = vec![b'X', b'0', b'C', 42u8];
        let (result, _) = get_tag_value(&bytes, 0).unwrap();
        assert_eq!(result, "X0:i:42");

        // 'S' (u16) should format type char as 'i'
        bytes = vec![b'X', b'1', b'S'];
        bytes.extend_from_slice(&100u16.to_le_bytes());
        let (result, _) = get_tag_value(&bytes, 0).unwrap();
        assert_eq!(result, "X1:i:100");

        // 'f' (f32) should format type char as 'i'
        bytes = vec![b'X', b'2', b'f'];
        bytes.extend_from_slice(&2.5f32.to_le_bytes());
        let (result, _) = get_tag_value(&bytes, 0).unwrap();
        assert_eq!(result, "X2:i:2.5");
    }
}

pub fn get_tag_value(bytes: &[u8], mut i: usize) -> Result<(String, usize), String> {
    let tag: String = bytes[i..i + 2].iter().map(|&b| b as char).collect();
    i += 2;

    let value_type = bytes[i];
    i += 1;


    let (result, i) = match value_type {
        b'B' => {
            let val_type = bytes[i];
            i += 1;

            let count = i32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|e| format!("Invalid count: {}", e))?);
            i += 4;

            let mut values = Vec::new();
            for _ in 0..count {
                let (value, new_i) = map_tag_type_to_result(bytes, i, val_type)?;
                values.push(value);
                i = new_i;
            }
            (values.join(","), i)
        }
        _ => map_tag_type_to_result(bytes, i, value_type)?,
    };

    let type_char = if "cCsSiIf".contains(value_type as char) {
        'i'
    } else {
        value_type as char
    };

    let formatted_result = format!("{}:{}:{}", tag, type_char, result);
    Ok((formatted_result, i))
}