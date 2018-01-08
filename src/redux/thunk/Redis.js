import { getAllKeysDone, getConfigDone, setConfigDone, getAllKeys, getConfig, getKeyValue, getKeyValueDone } from "../actions/Redis";
import { getActiveInstance } from "../../utils/InstanceUtil";

export function getCurrentInstanceKeys() {
    return dispatch => {
        dispatch(getAllKeys());
        try {
            getActiveInstance().keys("*", (error, res) => {
                dispatch(getAllKeysDone(res));
            })
        } catch (ex) {

        }
    }
}

export function getConfigByKey(config) {
    return dispatch => {
        dispatch(getConfig())
        try {
            getActiveInstance().config("GET", config, (err, res) => {
                if (res && res.length === 2) {
                    dispatch(getConfigDone({ key: config, value: res[1] }))
                }
            });
        } catch (ex) {
            console.log(ex);
        }
    }
}

export function setConfigByKey(action) {
    const { key, value } = action.config;
    //yield getActiveInstance().config("SET", key, value, (error, res) => {
    //    put(setConfigDone({ key: action.key, value: res }));
    //});
}

export function getObjectByKey(key) {
    return dispatch => {
        dispatch(getKeyValue(key));
        getActiveInstance().type(key, (error, t) => {
            if (t) {
                getActiveInstance().get(key, (err, r) => {
                    if (r) {
                        dispatch(getKeyValueDone({ key: key, value: r, type: t }));
                    }
                })
            }
        })
    }
}