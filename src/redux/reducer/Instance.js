import { INSTANCE_ADD, INSTANCE_DELETE, INSTANCE_GET, INSTANCE_GET_CONFIG, INSTANCE_SET_CONFIG } from '../actions/Instance';

export function handleInstance(state = {}, action) {
    switch (action.type) {
        case INSTANCE_ADD:
            return Object.assign({}, state, action.instance);
        case INSTANCE_GET:
            return Object.assign({}, state, action.instance);
        case INSTANCE_DELETE:
            return state;
        case INSTANCE_GET_CONFIG:
            return Object.assign({}, state, action.config);
        case INSTANCE_SET_CONFIG:
            return state;
        default:
            return state;
    }
}