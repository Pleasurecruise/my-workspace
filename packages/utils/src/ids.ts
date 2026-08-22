const createRandomId = () => {
	if (globalThis.crypto && "randomUUID" in globalThis.crypto) {
		return globalThis.crypto.randomUUID();
	}

	return `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 10)}`;
};

export const createPrefixedId = (prefix = "id") => `${prefix}_${createRandomId()}`;
