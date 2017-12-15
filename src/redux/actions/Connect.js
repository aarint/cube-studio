export const ACTION_CONNECT = "ACTION_CONNECT";
export const ACTION_DISCONNECT = "ACTION_DISCONNECT";
export const ACTION_UPDATE_CONNECT_STATUS = "ACTION_UPDATE_CONNECT_STATUS";

export function actionConnect(config) {
    return {
        type: ACTION_CONNECT,
        config
    }
}

export function actionDisconnect() {
    return {
        type: ACTION_DISCONNECT
    }
}

export function actionUpdateConnectStatus() {
    return {
        type: ACTION_UPDATE_CONNECT_STATUS
    }
}