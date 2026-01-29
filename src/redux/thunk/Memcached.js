import Memcached from 'node_memcached';
import {
    connectMemcachedDone,
    connectMemcachedError,
    getMemcachedKeys,
    getMemcachedKeysDone,
    getMemcachedValue,
    getMemcachedValueDone,
    setMemcachedValueDone,
    deleteMemcachedKeyDone
} from '../actions/Memcached';
import { addConnectedInstance, deleteInstance } from "../../utils/InstanceUtil";

// Singleton instance
let memcachedClient = null;

export function connectMemcached(config) {
    return async dispatch => {
        try {
            const location = `${config.ip}:${config.port}`;
            memcachedClient = new Memcached(location);

            memcachedClient.on('issue', (issue) => {
                console.error('Memcached issue:', issue);
                dispatch(connectMemcachedError(issue));
            });

            memcachedClient.on('failure', (details) => {
                console.error('Memcached failure:', details);
                dispatch(connectMemcachedError(details));
            });

            // Test connection
            await new Promise((resolve, reject) => {
                memcachedClient.get('test-connection', (err) => {
                    if (err && err.message !== 'Not Found') {
                        reject(err);
                    } else {
                        resolve();
                    }
                });
            });

            addConnectedInstance(config, memcachedClient);
            dispatch(connectMemcachedDone({
                ...config,
                type: 'Memcached',
                client: memcachedClient
            }));
        } catch (error) {
            console.error('Memcached connection error:', error);
            dispatch(connectMemcachedError(error));
            throw error;
        }
    };
}

export function getMemcachedKeysList() {
    return async dispatch => {
        dispatch(getMemcachedKeys());
        try {
            if (!memcachedClient) {
                return;
            }

            // Note: Memcached doesn't have a built-in "get all keys" command
            // We'll return an empty array for now
            // In a real implementation, you might need to use a separate keys tracking mechanism
            dispatch(getMemcachedKeysDone([]));
        } catch (error) {
            console.error('Error getting Memcached keys:', error);
        }
    };
}

export function getMemcachedObjectByKey(key) {
    return async dispatch => {
        dispatch(getMemcachedValue(key));
        try {
            if (!memcachedClient) {
                return;
            }

            memcachedClient.get(key, (err, data) => {
                if (err) {
                    console.error('Error getting Memcached value:', err);
                    return;
                }

                dispatch(getMemcachedValueDone({
                    key: key,
                    value: data ? data.toString() : null,
                    type: 'string'
                }));
            });
        } catch (error) {
            console.error('Error getting Memcached object:', error);
        }
    };
}

export function setMemcachedValue(key, value) {
    return async dispatch => {
        try {
            if (!memcachedClient) {
                return;
            }

            const lifetime = 3600; // 1 hour in seconds

            memcachedClient.set(key, value, lifetime, (err) => {
                if (err) {
                    console.error('Error setting Memcached value:', err);
                    return;
                }

                dispatch(setMemcachedValueDone({
                    key: key,
                    value: value,
                    type: 'string'
                }));
            });
        } catch (error) {
            console.error('Error setting Memcached value:', error);
        }
    };
}

export function deleteMemcachedValue(key) {
    return async dispatch => {
        try {
            if (!memcachedClient) {
                return;
            }

            memcachedClient.del(key, (err) => {
                if (err) {
                    console.error('Error deleting Memcached key:', err);
                    return;
                }

                dispatch(deleteMemcachedKeyDone(key));
            });
        } catch (error) {
            console.error('Error deleting Memcached key:', error);
        }
    };
}
