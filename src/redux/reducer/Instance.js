import { INSTANCE_ADD, INSTANCE_DELETE, INSTANCE_GET, INSTANCES_SAVED_GET_DONE } from '../actions/Instance';

const initialState = { instances: [] };

export function handleInstance(state = initialState, action) {
    switch (action.type) {
        case INSTANCE_ADD:
            return Object.assign({}, state, action.payload);
        case INSTANCE_GET:
            return Object.assign({}, state, action.payload);
        case INSTANCE_DELETE:
            return state;
        case INSTANCES_SAVED_GET_DONE:
            return Object.assign({}, state, {
                instances: action?.payload?.instances ?? action?.instances ?? []
            });
        default:
            return state;
    }
}