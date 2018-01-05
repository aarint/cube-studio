import { put, call } from 'redux-saga/effects';
import Redis from 'redis';
import { connectDBDone, connectDBError } from '../actions/Connect';
import { addConnectedInstance, deleteInstance } from "../../utils/InstanceUtil";

export function* connectDB(action) {
    const { config } = action;

    try {
        let client = Redis.createClient(config.port, config.ip);

        client.on('connect', function () {
            console.log('Client :: connect');
        })

        client.on('end', () => {
            console.log('Client :: end');
        })

        addConnectedInstance(config, client);
        yield put(connectDBDone(config));
    } catch (ex) {
        yield put(connectDBError(ex));
        console.log(ex);
    }
}

export function* disconnectDB(instance) {
    try {
        instance && instance.quit();
    } catch (ex) {
        console.log(ex);
    }
}

export function* removeDB(name, instance) {
    deleteInstance(name)
}