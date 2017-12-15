import { ACTION_CONNECT, ACTION_DISCONNECT, ACTION_UPDATE_CONNECT_STATUS } from '../actions/Connect';

const connectState = {
    msg: null
}

export function handleConnection(state = connectState, action) {
    switch (action.type) {
        case ACTION_CONNECT:
            return state;
        case ACTION_DISCONNECT:
            return state;
        case ACTION_UPDATE_CONNECT_STATUS:
            return state;
        default:
            return state;
    }
}