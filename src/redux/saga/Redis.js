import { put, call } from "redux-saga/effects";
import { addString, getString, getAllKeys } from "../actions/Redis";
import { getActiveInstance } from "../../utils/InstanceUtil";

export function* getCurrentInstanceKeys() {
    getActiveInstance().keys("*", function (erro, res) {
        console.log(res);
        //yield put(getAllKeys());
    })
}

export function* doingString() { }

export function* addStringByKeyValue(action) {
    try {
        yield put(addString());
        console.log('add',action);
        getActiveInstance().set(action.key, action.value);
    } catch (ex) {
        console.log(ex);
    }
}

export function* getStringByKey(action) {
    console.log('get', action.key)
}

export function* deleteString() { }