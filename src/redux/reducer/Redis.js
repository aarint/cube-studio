import {
    STRING_ADD,
    STRING_DELETE,
    STRING_GET
} from '../actions/Redis';

export function handleRedis(state = {}, action) {
    switch (action.type) {
        case STRING_ADD:
            return state;
        case STRING_DELETE:
            return state;
        case STRING_GET:
            return Object.assign({}, state, action.value);
        default:
            return state;
    }
}