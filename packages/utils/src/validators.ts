import validator from "validator";

/**
 * Email validation
 */
export const isEmail = (value: string): boolean => {
	return validator.isEmail(value);
};

/**
 * Mobile phone validation (supports multiple locales)
 */
export const isMobilePhone = (
	value: string,
	locale?: validator.MobilePhoneLocale | validator.MobilePhoneLocale[],
): boolean => {
	return validator.isMobilePhone(value, locale);
};

/**
 * China mainland mobile phone validation
 */
export const isChinaMobilePhone = (value: string): boolean => {
	return validator.isMobilePhone(value, "zh-CN");
};

/**
 * IP address validation (supports v4 and v6)
 */
export const isIP = (value: string, version?: 4 | 6): boolean => {
	return validator.isIP(value, version);
};

/**
 * IPv4 address validation
 */
export const isIPv4 = (value: string): boolean => {
	return validator.isIP(value, 4);
};

/**
 * IPv6 address validation
 */
export const isIPv6 = (value: string): boolean => {
	return validator.isIP(value, 6);
};

/**
 * URL validation
 */
export const isURL = (value: string, options?: validator.IsURLOptions): boolean => {
	return validator.isURL(value, options);
};

/**
 * UUID validation
 */
export const isUUID = (value: string, version?: 1 | 2 | 3 | 4 | 5): boolean => {
	return validator.isUUID(value, version);
};

/**
 * Identity card validation (China mainland by default)
 */
export const isIdentityCard = (value: string, locale?: validator.IdentityCardLocale): boolean => {
	return validator.isIdentityCard(value, locale ?? "zh-CN");
};

/**
 * Postal code validation
 */
export const isPostalCode = (value: string, locale: validator.PostalCodeLocale): boolean => {
	return validator.isPostalCode(value, locale);
};

/**
 * Credit card number validation
 */
export const isCreditCard = (value: string): boolean => {
	return validator.isCreditCard(value);
};

/**
 * JSON string validation
 */
export const isJSON = (value: string): boolean => {
	return validator.isJSON(value);
};

/**
 * Strong password validation
 */
export const isStrongPassword = (
	value: string,
	options?: validator.StrongPasswordOptions,
): boolean => {
	return Boolean(validator.isStrongPassword(value, options));
};

/**
 * Date string validation
 */
export const isDate = (value: string, options?: validator.IsDateOptions): boolean => {
	return validator.isDate(value, options);
};

/**
 * Numeric string validation
 */
export const isNumeric = (value: string): boolean => {
	return validator.isNumeric(value);
};

/**
 * Integer validation
 */
export const isInt = (value: string, options?: validator.IsIntOptions): boolean => {
	return Boolean(validator.isInt(value, options));
};

/**
 * Float validation
 */
export const isFloat = (value: string, options?: validator.IsFloatOptions): boolean => {
	return validator.isFloat(value, options);
};

/**
 * Hex color validation
 */
export const isHexColor = (value: string): boolean => {
	return validator.isHexColor(value);
};

/**
 * MAC address validation
 */
export const isMACAddress = (value: string): boolean => {
	return validator.isMACAddress(value);
};

/**
 * Alphanumeric validation
 */
export const isAlphanumeric = (value: string): boolean => {
	return validator.isAlphanumeric(value);
};

/**
 * Base64 validation
 */
export const isBase64 = (value: string): boolean => {
	return validator.isBase64(value);
};

/**
 * JWT validation
 */
export const isJWT = (value: string): boolean => {
	return validator.isJWT(value);
};

// Re-export original validator for additional features
export { default as validator } from "validator";
