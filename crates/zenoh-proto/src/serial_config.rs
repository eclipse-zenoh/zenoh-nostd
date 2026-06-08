#[derive(Clone, Debug, PartialEq)]
pub enum SerialParity {
    None,
    Even,
    Odd,
    Mark,
    Space,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SerialStopBits {
    One,
    OnePointFive,
    Two,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SerialDataBits {
    Five,
    Six,
    Seven,
    Eight,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SerialConfig {
    pub baud_rate: u32,
    pub data_bits: SerialDataBits,
    pub parity: SerialParity,
    pub stop_bits: SerialStopBits,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: SerialDataBits::Eight,
            parity: SerialParity::None,
            stop_bits: SerialStopBits::One,
        }
    }
}

pub fn parse_serial_config(address: &str) -> SerialConfig {
    let mut config = SerialConfig::default();

    if let Some(query_start) = address.find('?') {
        let query = &address[query_start + 1..];
        for param in query.split('&') {
            if let Some(eq) = param.find('=') {
                let key = &param[..eq];
                let value = &param[eq + 1..];
                match key {
                    "baud" => {
                        if let Ok(baud) = value.parse::<u32>() {
                            config.baud_rate = baud;
                        }
                    }
                    "parity" => {
                        config.parity = match value {
                            "even" => SerialParity::Even,
                            "odd" => SerialParity::Odd,
                            "mark" => SerialParity::Mark,
                            "space" => SerialParity::Space,
                            _ => SerialParity::None,
                        };
                    }
                    "bits" => {
                        config.data_bits = match value {
                            "5" => SerialDataBits::Five,
                            "6" => SerialDataBits::Six,
                            "7" => SerialDataBits::Seven,
                            _ => SerialDataBits::Eight,
                        };
                    }
                    "stop" => {
                        config.stop_bits = match value {
                            "1" => SerialStopBits::One,
                            "1.5" => SerialStopBits::OnePointFive,
                            "2" => SerialStopBits::Two,
                            _ => SerialStopBits::One,
                        };
                    }
                    _ => {}
                }
            }
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_serial_config_defaults() {
        let config = parse_serial_config("/dev/ttyUSB0");
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.parity, SerialParity::None);
        assert_eq!(config.data_bits, SerialDataBits::Eight);
        assert_eq!(config.stop_bits, SerialStopBits::One);
    }

    #[test]
    fn test_parse_serial_config_full() {
        let config = parse_serial_config("/dev/ttyUSB0?baud=9600&parity=even&bits=7&stop=2");
        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.parity, SerialParity::Even);
        assert_eq!(config.data_bits, SerialDataBits::Seven);
        assert_eq!(config.stop_bits, SerialStopBits::Two);
    }

    #[test]
    fn test_parse_serial_config_partial() {
        let config = parse_serial_config("/dev/ttyS0?baud=57600");
        assert_eq!(config.baud_rate, 57600);
        assert_eq!(config.parity, SerialParity::None);
        assert_eq!(config.data_bits, SerialDataBits::Eight);
        assert_eq!(config.stop_bits, SerialStopBits::One);
    }

    #[test]
    fn test_parse_serial_config_odd_parity() {
        let config = parse_serial_config("/dev/ttyUSB0?parity=odd");
        assert_eq!(config.parity, SerialParity::Odd);
    }

    #[test]
    fn test_parse_serial_config_mark_parity() {
        let config = parse_serial_config("/dev/ttyUSB0?parity=mark");
        assert_eq!(config.parity, SerialParity::Mark);
    }

    #[test]
    fn test_parse_serial_config_space_parity() {
        let config = parse_serial_config("/dev/ttyUSB0?parity=space");
        assert_eq!(config.parity, SerialParity::Space);
    }

    #[test]
    fn test_parse_serial_config_stop_bits_one_point_five() {
        let config = parse_serial_config("/dev/ttyUSB0?stop=1.5");
        assert_eq!(config.stop_bits, SerialStopBits::OnePointFive);
    }

    #[test]
    fn test_parse_serial_config_data_bits_5() {
        let config = parse_serial_config("/dev/ttyUSB0?bits=5");
        assert_eq!(config.data_bits, SerialDataBits::Five);
    }

    #[test]
    fn test_parse_serial_config_data_bits_6() {
        let config = parse_serial_config("/dev/ttyUSB0?bits=6");
        assert_eq!(config.data_bits, SerialDataBits::Six);
    }

    #[test]
    fn test_parse_serial_config_invalid_baud() {
        let config = parse_serial_config("/dev/ttyUSB0?baud=invalid");
        assert_eq!(config.baud_rate, 115200);
    }

    #[test]
    fn test_parse_serial_config_invalid_parity() {
        let config = parse_serial_config("/dev/ttyUSB0?parity=invalid");
        assert_eq!(config.parity, SerialParity::None);
    }

    #[test]
    fn test_parse_serial_config_invalid_stop_bits() {
        let config = parse_serial_config("/dev/ttyUSB0?stop=3");
        assert_eq!(config.stop_bits, SerialStopBits::One);
    }

    #[test]
    fn test_parse_serial_config_invalid_data_bits() {
        let config = parse_serial_config("/dev/ttyUSB0?bits=9");
        assert_eq!(config.data_bits, SerialDataBits::Eight);
    }
}