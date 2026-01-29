import {
    CONNECT_MEMCACHED_DONE,
    CONNECT_MEMCACHED_ERROR,
    GET_MEMCACHED_KEYS_DONE,
    GET_MEMCACHED_VALUE_DONE,
    SET_MEMCACHED_VALUE_DONE,
    DELETE_MEMCACHED_KEY_DONE
} from '../actions/Memcached';

const initialState = {
    client: null,
    config: null,
    keys: [],
    obj: null,
    error: null,
    isChanging: false
};

export default function memcached(state = initialState, action) {
    switch (action.type) {
        case CONNECT_MEMCACHED_DONE:
            return {
                ...state,
                client: action.payload.client,
                config: action.payload,
                error: null
            }
        case CONNECT_MEMCACHED_ERROR:
            return {
                ...state,
                error: action.payload
            }
        case GET_MEMCACHED_KEYS_DONE:
            return {
                ...state,
                keys: action.payload,
                isChanging: false
            }
        case GET_MEMCACHED_VALUE_DONE:
            return {
                ...state,
                obj: action.payload
            }
        case SET_MEMCACHED_VALUE_DONE:
            return {
                ...state,
                obj: action.payload
            }
        case DELETE_MEMCACHED_KEY_DONE:
            return {
                ...state,
                keys: state.keys.filter(k => k !== action.payload),
                obj: null
            }
        default:
            return state;
    }
}
