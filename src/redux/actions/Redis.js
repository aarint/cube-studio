export const STRING_DOING = "STRING_DOING";
export const STRING_ADD = "STRING_ADD";
export const STRING_DELETE = "STRING_DELETE";

export const KEYS_GET = "KEYS_GET";
export const KEYS_GET_DONE = "KEYS_GET_DONE";

export const KEY_VALUE_GET = "KEY_VALUE_GET";
export const KEY_VALUE_GET_DONE = "KEY_VALUE_GET_DONE";

export const CONFIG_GET = "CONFIG_GET";
export const CONFIG_GET_DONE = "CONFIG_GET_DONE";

export const CONFIG_SET = "CONFIG_SET";
export const CONFIG_SET_DONE = "CONFIG_SET_DONE";

export const CHANGE_DB = "CHNAGE_DB";
export const CHANGE_DB_DONE = "CHANGE_DB_DONE";

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

export function getAllKeys(db) {
    return {
        type: KEYS_GET,
        db
    }
}

export function getAllKeysDone(keys) {
    return {
        type: KEYS_GET_DONE,
        keys
    }
}

export function getConfig(key) {
    return {
        type: CONFIG_GET,
        key
    }
}

export function getConfigDone(config) {
    return {
        type: CONFIG_GET_DONE,
        config
    }
}

export function setConfig(key, value) {
    return {
        type: CONFIG_SET,
        key,
        value
    }
}

export function setConfigDone(config) {
    return {
        type: CONFIG_SET_DONE,
        config
    }
}

export function getKeyValue(key) {
    return {
        type: KEY_VALUE_GET,
        key
    }
}

export function getKeyValueDone(obj) {
    return {
        type: KEY_VALUE_GET_DONE,
        obj
    }
}

export function changeDB(db) {
    return {
        type: CHANGE_DB,
        db
    }
}

export function changeDBDone(db) {
    return {
        type: CHANGE_DB_DONE,
        db
    }
}