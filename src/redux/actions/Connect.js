export const ACTION_CONNECT = "ACTION_CONNECT";
export const ACTION_DISCONNECT = "ACTION_DISCONNECT";

export function actionConnect() {
    return {
        type: ACTION_CONNECT,
        instance
    }
}

export function actionDisconnect(instance) {
    return {
        type: ACTION_DISCONNECT,
        instance
    }
}