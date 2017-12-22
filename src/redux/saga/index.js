import { takeLatest } from 'redux-saga/effects';
import { ACTION_CONNECT, ACTION_DISCONNECT, ACTION_UPDATE_CONNECT_STATUS } from "../actions/Connect";
import { INSTANCE_ADD, INSTANCE_DELETE, INSTANCE_GET } from "../actions/Instance";
import { connectDB, disconnectDB } from './Connect';
import { addInstance, getInstance, deleteInstance, getConfig } from './Instance';

export default function* index() {
    yield [
        takeLatest(ACTION_CONNECT, connectDB),
        takeLatest(ACTION_DISCONNECT,disconnectDB),
        takeLatest(INSTANCE_ADD,addInstance),
        takeLatest(INSTANCE_DELETE,deleteInstance),
        takeLatest(INSTANCE_GET,getInstance)
    ]
}