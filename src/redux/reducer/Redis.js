import {
    STRING_ADD,
    STRING_DELETE,
    STRING_GET
} from '../actions/Redis';

const min = {
    msg: null
}

export function handleRedis(state = min, action) {
    switch (action.type) {
        case STRING_ADD:
            return state;
        case STRING_DELETE:
            return state;
        case STRING_GET:
            return state;
        default:
            return state;
    }
}