export const STRING_DOING = "STRING_DOING";
export const STRING_ADD = "STRING_ADD";
export const STRING_DELETE = "STRING_DELETE";
export const STRING_GET = "STRING_GET";

export const HASH_ADD = "";
export const HASH_DLETE = "";
export const HASH_GET = "";

export const LIST_SET = "";
export const LIST_GET = "";

export const SET_ADD = "";

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