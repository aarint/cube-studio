import { STRING_ADD, STRING_DELETE, CONFIG_GET_DONE, CONFIG_SET_DONE, KEYS_GET_DONE, KEY_VALUE_GET_DONE } from '../actions/Redis';

export function handleRedis(state = {}, action) {
    switch (action.type) {
        case STRING_ADD:
            return Object.assign({}, state, { isAdding: false });
        case STRING_DELETE:
            return state;
        case KEY_VALUE_GET_DONE:
            return Object.assign({}, state, { obj: action.obj });
        case KEYS_GET_DONE:
            return Object.assign({}, state, { keys: action.keys });
        case CONFIG_GET_DONE:
            return Object.assign({}, state, { config: action.config });
        case CONFIG_SET_DONE:
            return Object.assign({}, state, { config: action.config });
        default:
            return state;
    }
}