import { put, call } from "redux-saga/effects";
import { addString, getString, getAllKeysDone, getConfigDone, setConfigDone, getKeyValueDone } from "../actions/Redis";
import { getActiveInstance } from "../../utils/InstanceUtil";

export function* getCurrentInstanceKeys(action) {
    const keys = yield new Promise((resolve, reject) => {
        getActiveInstance().keys("*", (error, res) => {
            if (res && res.length > 0) {
                resolve(res);
            } else {
                reject(null);
            }
        })
    })

    yield put(getAllKeysDone(keys));
}

export function* doingString() { }

export function* addStringByKeyValue(action) {
    try {
        yield put(addString(action.key, action.value));
        yield getActiveInstance().set(action.key, action.value);
    } catch (ex) {
        console.log(ex);
    }
}

export function* getStringByKey(action) {
    console.log('get', action.key)
}

export function* deleteStringByKey() { }

export function* getConfigByKey(action) {
    const value = yield new Promise((resolve, reject) => {
        getActiveInstance().config("GET", action.key, (error, res) => {
            if (res && res.length === 2) {
                resolve(res[1])
            } else {
                reject(null);
            }
        });
    })

    yield put(getConfigDone({ key: action.key, value: value }));
}

export function* setConfigByKey(action) {
    const { key, value } = action.config;
    yield getActiveInstance().config("SET", key, value, (error, res) => {
        put(setConfigDone({ key: action.key, value: res }));
    });
}

export function* getObjectByKey(action) {
    const { key } = action.key;
    const result = yield new Promise((resolve, reject) => {
        getActiveInstance().type(action.key, (error, t) => {
            if (t) {
                getActiveInstance().get(action.key, (err, r) => {
                    if (r) {
                        resolve({ key: action.key, value: r, type: t });
                    } else {
                        reject(null);
                    }
                })
            } else {
                reject(null);
            }
        })
    })
    
    yield put(getKeyValueDone(result));
}

function getKeyValue(type, key, resovle, reject) {
    switch (type.toUpper) {
        case "STRING":
            getActiveInstance().get(key);
            break;
    }
}