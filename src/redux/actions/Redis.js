export const STRING_ADD = "INSTANCE_SET";
export const STRING_DELETE = "";
export const STRING_GET = "INSTANCE_GET";

export const HASH_ADD = "";
export const HASH_DLETE = "";
export const HASH_GET = "";

export const LIST_SET = "";
export const LIST_GET = "";

export const SET_ADD = "";

export function addString(key, value) {
    return {
        type: INSTANCE_SET_KEY,
        key,
        value
    }
}

export function getString(key) {
    return {
        type: INSTANCE_GET_KEY,
        key
    }
}
