export const CONNECT_START = "CONNECT_START";
export const CONNECT_DONE = "CONNECT_DONE";
export const CONNECT_ERROR = "CONNECT_ERROR";
export const DISCONNECT = "DISCONNECT";

export function connectStart() {
    return {
        type: CONNECT_START
    }
}

export function connectDone(client) {
    return {
        type: CONNECT_DONE,
        client
    }
}

export function connectError(msg) {
    return {
        type: CONNECT_ERROR,
        msg
    }
}

export function disconnect() {
    return {
        type: DISCONNECT
    }
}