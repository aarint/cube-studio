export const CONNECT_DB = "CONNECT_DB";
export const CONNECT_DB_START = "CONNECT_START";
export const CONNECT_DB_DONE = "CONNECT_DONE";
export const CONNECT_DB_ERROR = "CONNECT_ERROR";
export const DISCONNECT_DB = "DISCONNECT";

export function connectDB() {
    return {
        type: CONNECT_DB
    }
}

export function connectDBStart() {
    return {
        type: CONNECT_DB_START
    }
}

export function connectDBDone(client) {
    return {
        type: CONNECT_DB_DONE,
        client
    }
}

export function connectDBError(msg) {
    return {
        type: CONNECT_DB_ERROR,
        msg
    }
}

export function disconnectDB() {
    return {
        type: DISCONNECT_DB
    }
}