
pub const VALID_PHONE_NUMBER: &str = r"^\+?[0-9]{1,3}\-?[0-9]{3,14}$";

pub const VALID_EMAIL: &str = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";

pub const VALID_PASSWORD: &str = r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)[a-zA-Z\d]{8,}$";

pub const VALID_USERNAME: &str = r"^[a-zA-Z0-9_]{3,15}$";

pub const VALID_JWT: &str = r"^[A-Za-z0-9-_=]+\.[A-Za-z0-9-_=]+\.?[A-Za-z0-9-_.+/=]*$";

pub const VALID_UUID: &str = r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";

pub const VALID_URL: &str = r"^http?://[^\s/$.?#].[^\s]*$";

pub const VALID_IP: &str = r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$";

pub const VALID_IPV4: &str = r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$";

pub const VALID_IPV6: &str = r"^(?:(?:(?:[0-9a-fA-F]){1,4}:){7}(?:[0-9a-fA-F]){1,4})|(?:(?:[0-9a-fA-F]){1,4}:){6}(?:(?:(?:[0-9a-fA-F]){1,4}:)?(?:[0-9a-fA-F]){1,4}:[0-9a-fA-F]{1,4})|(?:(?:[0-9a-fA-F]){1,4}:){5}(?:(?:(?:[0-9a-fA-F]){1,4}:)?(?:[0-9a-fA-F]){1,4}:[0-9a-fA-F]{1,4}|(?:(?:[0-9a-fA-F]){1,4}:)";

pub const VALID_MAC_ADDRESS: &str = r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$";

pub const VALID_MD5: &str = r"^[0-9a-fA-F]{32}$";

pub const VALID_SHA1: &str = r"^[0-9a-fA-F]{40}$";

pub const VALID_SHA256: &str = r"^[0-9a-fA-F]{64}$";

pub const VALID_SHA512: &str = r"^[0-9a-fA-F]{128}$";

pub const VALID_UUID_V4: &str = r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";

pub const VALID_UUID_V5: &str = r"^[0-9a-f]{8}-[0-9a-f]{4}-5[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";

pub const VALID_ISBN: &str = r"^(?:ISBN(?:-1[03])?:? )?(?=[-0-9X]{13}$|[-0-9+X]{17}$|97[89][0-9]{10}$|97[89][0-9]{13}$)(?:97[89][- ]?)?[0-9]{1,5}[- ]?([0-9]+[- ]?){2}[0-9X]$";

pub const VALID_ISBN10: &str = r"^(?:ISBN(?:-1[03])?:? )?(?=[-0-9X]{13}$|[-0-9+X]{17}$|97[89][0-9]{10}$|97[89][0-9]{13}$)(?:97[89][- ]?)?[0-9]{1,5}[- ]?([0-9]+[- ]?){2}[0-9X]$";

pub const VALID_ISBN13: &str = r"^(?:ISBN(?:-1[03])?:? )?(?=[-0-9X]{13}$|[-0-9+X]{17}$|97[89][0-9]{10}$|97[89][0-9]{13}$)(?:97[89][- ]?)?[0-9]{1,5}[- ]?([0-9]+[- ]?){2}[0-9X]$";

pub const VALID_EAN: &str = r"^(?:[0-9]{13})|(?:[0-9]{8})$";

pub const VALID_EAN13: &str = r"^(?:[0-9]{13})$";

pub const VALID_EAN8: &str = r"^(?:[0-9]{8})$";

pub const VALID_UPC: &str = r"^(?:[0-9]{12})|(?:[0-9]{11})|(?:[0-9]{10})|(?:[0-9]{7})|(?:[0-9]{6})|(?:[0-9]{5})|(?:[0-9]{4})|(?:[0-9]{3})|(?:[0-9]{2})$";

pub const VALID_UPC_A: &str = r"^(?:[0-9]{12})$";

pub const VALID_UPC_E: &str = r"^(?:[0-9]{11})|(?:[0-9]{10})|(?:[0-9]{7})|(?:[0-9]{6})|(?:[0-9]{5})|(?:[0-9]{4})|(?:[0-9]{3})|(?:[0-9]{2})$";

pub const VALID_ISIN: &str = r"^([A-Z]{2})([0-9A-Z]{9})([0-9])$";

pub const VALID_ISMN: &str = r"^(?:979-?)?[0-9]{9}[0-9Xx]$";

pub const VALID_ISSN: &str = r"^[0-9]{4}-[0-9]{3}[0-9Xx]$";

pub const VALID_ISRC: &str = r"^[A-Z]{2}[0-9A-Z]{3}[0-9]{7}$";

pub const VALID_ISSN13: &str = r"^[0-9]{4}-[0-9]{3}[0-9Xx]$";

pub const VALID_ISSN10: &str = r"^[0-9]{4}-[0-9]{3}[0-9]$";

pub const VALID_NPI: &str = r"^[0-9]{10}$";

pub const VALID_SSN: &str = r"^\d{3}-\d{2}-\d{4}$";

pub const VALID_TAX_ID: &str = r"^[0-9]{3}-[0-9]{2}-[0-9]{4}$";

pub const VALID_VAT_ID: &str = r"^[0-9]{2,12}$";

pub const VALID_IBAN: &str = r"^[A-Z]{2}\d{2}(?:\s*\d{4}){6,7}(?:\s*[A-Z0-9]{3})?$";

pub const VALID_BIC: &str = r"^[A-Z]{6}[A-Z2-9][A-NP-Z0-9](?:[A-Z0-9]{3})?$";

pub const VALID_BIC_CODE: &str = r"^[A-Z]{6}[A-Z2-9][A-NP-Z0-9](?:[A-Z0-9]{3})?$";

pub const VALID_BANK_ACCOUNT_NUMBER: &str = r"^[0-9]{8,17}$";

pub const VALID_IBAN_CODE: &str = r"^[A-Z]{2}\d{2}(?:\s*\d{4}){6,7}(?:\s*[A-Z0-9]{3})?$";

pub const VALID_IBAN_ACCOUNT_NUMBER: &str = r"^[0-9]{8,17}$";

