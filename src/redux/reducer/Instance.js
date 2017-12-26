import { INSTANCE_ADD, INSTANCE_DELETE, INSTANCE_GET } from '../actions/Instance';

export function handleInstance(state = {}, action) {
    switch (action.type) {
        case INSTANCE_ADD:
            return Object.assign({}, state, action.instance);
        case INSTANCE_GET:
            return Object.assign({}, state, action.instance);
        case INSTANCE_DELETE:
            return state;
        default:
            return state;
    }
}