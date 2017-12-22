import { ACTION_CONNECT, ACTION_DISCONNECT } from '../actions/Connect';

const connectState = {
    msg: null
}

export function handleConnection(state = connectState, action) {
    switch (action.type) {
        case ACTION_CONNECT:
            return Object.assign({},state,action.instance);
        case ACTION_DISCONNECT:
            return state;
        default:
            return state;
    }
}