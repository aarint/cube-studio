import { CONNECT_DB_DONE, CONNECT_DB_ERROR, DISCONNECT_DB_DONE, DISCONNECT_DB_ERROR } from '../actions/Connect';

export function handleConnection(state = {}, action) {
    switch (action.type) {
        case CONNECT_DB_DONE:
            return Object.assign({}, state, { result: action.result });
        case CONNECT_DB_ERROR:
            return Object.assign({}, state, { msg: action.msg });
        case DISCONNECT_DB_DONE:
            return state;
        default:
            return state;
    }
}
