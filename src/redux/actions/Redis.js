export const STRING_DOING = "STRING_DOING";
export const STRING_ADD = "STRING_ADD";
export const STRING_DELETE = "STRING_DELETE";
export const STRING_GET = "STRING_GET";

export const KEYS = "KEYS";

export function doingString() {
    return {
        type: STRING_DOING
    }
}

export function addString(key, value) {
    return {
        type: STRING_ADD,
        key,
        value
    }
}

export function deleteString(key) {
    return {
        type: STRING_DELETE,
        key
    }
}

export function getString(value) {
    return {
        type: STRING_GET,
        value
    }
}

export function getAllKeys(keys) {
    return {
        type: KEYS,
        keys
    }
}