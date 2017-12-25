import { STRING_ADD, STRING_DELETE, STRING_GET, KEYS } from '../actions/Redis';

export function handleRedis(state = {}, action) {
    switch (action.type) {
        case STRING_ADD:
            return Object.assign({}, state, { isAdding: false });
        case STRING_DELETE:
            return state;
        case STRING_GET:
            return Object.assign({}, state, action.value);
        case KEYS:
            return Object.assign({}, state, { keys: action.keys })
        default:
            return state;
    }
}