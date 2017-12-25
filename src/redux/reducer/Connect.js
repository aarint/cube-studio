import { CONNECT_DB, CONNECT_DB_START, CONNECT_DB_DONE, CONNECT_DB_ERROR, DISCONNECT_DB } from '../actions/Connect';

export function handleConnection(state = {}, action) {
    switch (action.type) {
        case CONNECT_DB:
            return Object.assign({}, state, { connected: false });
        case CONNECT_DB_START:
            return state;
        case CONNECT_DB_DONE:
            return Object.assign({}, state, { client: action.client, connected: true });
        case CONNECT_DB_ERROR:
            return Object.assign({}, state, { msg: action.msg });
        default:
            return state;
    }
}

export function handleDisconnection(state = {}, action) {
    switch (action.type) {
        case DISCONNECT_DB:
            return state;
        default:
            return state;
    }
}