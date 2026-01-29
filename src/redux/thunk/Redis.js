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
        dispatch(getConfig());
        try {
            const instance = getActiveInstance();
            const res = await instance.config('GET', config);
            if (res && res.length === 2) {
                dispatch(getConfigDone({ key: config, value: res[1] }));
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
            await instance.config('SET', key, value);
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

