import { put, call } from "redux-saga/effects";
import { addInstance, getInstance } from "../actions/Instance";
import { addNewInstance, deleteInstanceByKey } from '../../utils/InstanceUtil';

export function* addConnectedInstance(action) {
    try {
        console.log(action);

        //yield put(addInstance(action.instance));
        //addNewInstance(action.key, action.instance);
    } catch (ex) {
        deleteInstanceByKey(action.key);
        console.log('Wrong with adding aninstance');
    }
}

export function* getAllConnectedInstances() {
    try {
    } catch (ex) {
        console.log("Wrong with getting all instances.");
        return null;
    }
}

export function* getConnectedInstance(key) {
    try {
    } catch (ex) {
        console.log("Wrong with getting an instance.");
        return null;
    }
}

export function* deleteConnectedInstance(key) {
    try {
    } catch (ex) {
        console.log("Wrong with deleting an instance.");
    }
}

export function* getConnectedConfig(key) {
    console.log('get current connnected instance config!!!');
}