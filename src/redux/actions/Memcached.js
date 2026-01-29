// Memcached Actions

export const CONNECT_MEMCACHED_DONE = 'CONNECT_MEMCACHED_DONE';
export const CONNECT_MEMCACHED_ERROR = 'CONNECT_MEMCACHED_ERROR';

export const GET_MEMCACHED_KEYS = 'GET_MEMCACHED_KEYS';
export const GET_MEMCACHED_KEYS_DONE = 'GET_MEMCACHED_KEYS_DONE';

export const GET_MEMCACHED_VALUE = 'GET_MEMCACHED_VALUE';
export const GET_MEMCACHED_VALUE_DONE = 'GET_MEMCACHED_VALUE_DONE';

export const SET_MEMCACHED_VALUE = 'SET_MEMCACHED_VALUE';
export const SET_MEMCACHED_VALUE_DONE = 'SET_MEMCACHED_VALUE_DONE';

export const DELETE_MEMCACHED_KEY = 'DELETE_MEMCACHED_KEY';
export const DELETE_MEMCACHED_KEY_DONE = 'DELETE_MEMCACHED_KEY_DONE';

export function connectMemcachedDone(config) {
    return {
        type: CONNECT_MEMCACHED_DONE,
        payload: config
    }
}

export function connectMemcachedError(error) {
    return {
        type: CONNECT_MEMCACHED_ERROR,
        payload: error
    }
}

export function getMemcachedKeys() {
    return {
        type: GET_MEMCACHED_KEYS
    }
}

export function getMemcachedKeysDone(keys) {
    return {
        type: GET_MEMCACHED_KEYS_DONE,
        payload: keys
    }
}

export function getMemcachedValue(key) {
    return {
        type: GET_MEMCACHED_VALUE,
        payload: key
    }
}

export function getMemcachedValueDone(data) {
    return {
        type: GET_MEMCACHED_VALUE_DONE,
        payload: data
    }
}

export function setMemcachedValueDone(data) {
    return {
        type: SET_MEMCACHED_VALUE_DONE,
        payload: data
    }
}

export function deleteMemcachedKeyDone(key) {
    return {
        type: DELETE_MEMCACHED_KEY_DONE,
        payload: key
    }
}
