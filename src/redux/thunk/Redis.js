import { getAllKeysDone, getConfigDone, setConfigDone, getAllKeys, getConfig, getKeyValue, getKeyValueDone, changeDB, changeDBDone } from "../actions/Redis";
import { getActiveInstance } from "../../utils/InstanceUtil";

export function getCurrentInstanceKeys() {
    return async dispatch => {
        dispatch(getAllKeys());
        try {
            const instance = getActiveInstance();
            const keys = await instance.keys('*');
            dispatch(getAllKeysDone(keys));
        } catch (ex) {
            console.error('Error getting keys:', ex);
        }
    }
}

export function getConfigByKey(config) {
    return async dispatch => {
        dispatch(getConfig(config));
        try {
            const instance = getActiveInstance();
            // redis@4: configGet returns object, e.g. { databases: "16" }
            const res = await instance.configGet(config);
            const value = res ? res[config] : undefined;
            if (value !== undefined) {
                dispatch(getConfigDone({ key: config, value }));
            }
        } catch (ex) {
            console.error('Error getting config:', ex);
        }
    }
}

export function setConfigByKey(action) {
    const { key, value } = action.config;
    return async dispatch => {
        try {
            const instance = getActiveInstance();
            await instance.configSet({ [key]: value });
            dispatch(setConfigDone({ key: action.key, value: value }));
        } catch (error) {
            console.error('Error setting config:', error);
        }
    };
}

export function getObjectByKey(key) {
    return async dispatch => {
        dispatch(getKeyValue(key));
        try {
            const instance = getActiveInstance();
            const type = await instance.type(key);
            if (type) {
                const value = await instance.get(key);
                if (value) {
                    dispatch(getKeyValueDone({ key: key, value: value, type: type }));
                }
            }
        } catch (error) {
            console.error('Error getting object:', error);
        }
    }
}

export function setKeyValue(key, value) {
    return async (dispatch) => {
        try {
            const instance = getActiveInstance();
            await instance.set(key, value);
            const keys = await instance.keys('*');
            dispatch(getAllKeysDone(keys));
            dispatch(getKeyValueDone({ key, value, type: 'string' }));
        } catch (error) {
            console.error('Error setting key/value:', error);
            throw error;
        }
    }
}

export function deleteKey(key) {
    return async (dispatch) => {
        try {
            const instance = getActiveInstance();
            await instance.del(key);
            const keys = await instance.keys('*');
            dispatch(getAllKeysDone(keys));
        } catch (error) {
            console.error('Error deleting key:', error);
            throw error;
        }
    }
}

export function changeDataBase(key) {
    return async dispatch => {
        dispatch(changeDB(key));
        try {
            const instance = getActiveInstance();
            await instance.select(key);
            const keys = await instance.keys('*');
            dispatch(changeDBDone(key));
            dispatch(getAllKeysDone(keys));
        } catch (error) {
            console.error('Error changing database:', error);
        }
    }
}

