export const CONNECT_DB = "CONNECT_DB";
export const CONNECT_DB_DONE = "CONNECT_DONE";
export const CONNECT_DB_ERROR = "CONNECT_ERROR";

export const DISCONNECT_DB = "DISCONNECT_DB";
export const DISCONNECT_DB_DONE = "DISCONNECT_DB_DONE";
export const DISCONNECT_DB_ERROR = "DISCONNECT_DB_ERROR";

export function connectDB(config) {
    return {
        type: CONNECT_DB,
        config
    }
}

export function connectDBDone(result) {
    return {
        type: CONNECT_DB_DONE,
        result
    }
}

export function connectDBError(msg) {
    return {
        type: CONNECT_DB_ERROR,
        msg
    }
}

export function disconnectDB(config) {
    return {
        type: DISCONNECT_DB,
        config
    }
}

export function disconnectDBDone() {
    return {
        type: DISCONNECT_DB_DONE
    }
}

export function disconnectDBError() {
    return {
        type: DISCONNECT_DB_ERROR
    }
}