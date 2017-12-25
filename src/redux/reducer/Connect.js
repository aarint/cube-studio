import { CONNECT_START, CONNECT_DONE, CONNECT_ERROR, DISCONNECT } from '../actions/Connect';

export function handleConnection(state = {}, action) {
    switch (action.type) {
        case CONNECT_START:
            return state;
        case CONNECT_DONE:
            return Object.assign({}, state, action.client);
        case CONNECT_ERROR:
            return Object.assign({}, state, action.msg);
        default:
            return state;
    }
}

export function handleDisconnection(state = {}, action) {
    switch (action.type) {
        case DISCONNECT:
            return state;
        default:
            return state;
    }
}