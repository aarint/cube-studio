const STORAGE_CRYPTO_PREFIX = 'cs.v1:';

function getCrypto() {
    try {
        // Electron renderer (nodeIntegration) / dev env
        // eslint-disable-next-line global-require
        return require('crypto');
    } catch (e) {
        return null;
    }
}

function deriveKey(crypto) {
    // NOTE: This is application-level obfuscation to protect casual inspection.
    // For stronger security, use a user-provided passphrase or OS keychain.
    const secret = 'cube-studio.local-storage';
    const salt = 'cube-studio.salt.v1';
    return crypto.scryptSync(secret, salt, 32);
}

export function encryptToStorageString(obj) {
    const crypto = getCrypto();
    if (!crypto) {
        // Fallback: plaintext JSON (still returns a string)
        return STORAGE_CRYPTO_PREFIX + Buffer.from(JSON.stringify(obj), 'utf8').toString('base64');
    }

    const key = deriveKey(crypto);
    const iv = crypto.randomBytes(12);
    const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);

    const plaintext = Buffer.from(JSON.stringify(obj), 'utf8');
    const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
    const tag = cipher.getAuthTag();

    const payload = {
        a: 'aes-256-gcm',
        iv: iv.toString('base64'),
        tag: tag.toString('base64'),
        data: ciphertext.toString('base64'),
    };

    return STORAGE_CRYPTO_PREFIX + Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
}

export function decryptFromStorageString(str) {
    if (!str) return null;

    if (!str.startsWith(STORAGE_CRYPTO_PREFIX)) {
        // legacy plaintext JSON
        try {
            return JSON.parse(str);
        } catch (e) {
            return null;
        }
    }

    const b64 = str.slice(STORAGE_CRYPTO_PREFIX.length);
    let decoded;
    try {
        decoded = Buffer.from(b64, 'base64').toString('utf8');
    } catch (e) {
        return null;
    }

    // Fallback branch (no crypto): stored as base64(JSON)
    // If decoded is not an object payload, treat as JSON directly.
    let payload;
    try {
        payload = JSON.parse(decoded);
    } catch (e) {
        return null;
    }

    // If payload doesn't look like encrypted payload, treat it as the raw object.
    if (!payload || !payload.iv || !payload.tag || !payload.data) {
        return payload;
    }

    const crypto = getCrypto();
    if (!crypto) {
        return null;
    }

    try {
        const key = deriveKey(crypto);
        const iv = Buffer.from(payload.iv, 'base64');
        const tag = Buffer.from(payload.tag, 'base64');
        const data = Buffer.from(payload.data, 'base64');

        const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv);
        decipher.setAuthTag(tag);
        const plaintext = Buffer.concat([decipher.update(data), decipher.final()]);
        return JSON.parse(plaintext.toString('utf8'));
    } catch (e) {
        return null;
    }
}

