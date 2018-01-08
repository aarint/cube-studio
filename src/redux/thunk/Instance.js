import { getSavedInstances } from '../../utils/InstanceUtil';
import { addInstance, getInstance, getSavedInstancesDone } from "../actions/Instance";

export function addConnectedInstance(action) {
    return dispatch => {
        try {
            dispatch(addInstance(action.instance));
            addNewInstance(action.key, action.instance);
        } catch (ex) {
            console.log('Wrong with adding aninstance');
        }
    }
}

export function getAllSavedInstances() {
    return dispatch => {
        try {
            const instances = getSavedInstances();
            dispatch(getSavedInstancesDone(instances));
        } catch (ex) {
            console.log(ex);
        }
    }
}

export function getConnectedInstance(key) {
    try {
    } catch (ex) {
        console.log("Wrong with getting an instance.");
        return null;
    }
}

export function deleteConnectedInstance(key) {
    try {
    } catch (ex) {
        console.log("Wrong with deleting an instance.");
    }
}

export function getConnectedConfig(key) {
    console.log('get current connnected instance config!!!');
}
