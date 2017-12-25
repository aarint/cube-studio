import { takeLatest } from 'redux-saga/effects';
import { CONNECT_START, CONNECT_DONE, CONNECT_ERROR, DISCONNECT } from "../actions/Connect";
import { INSTANCE_ADD, INSTANCE_DELETE, INSTANCE_GET } from "../actions/Instance";
import { STRING_DOING, STRING_ADD, STRING_GET, STRING_DELETE } from "../actions/Redis";
import { connectDB, disconnectDB } from './Connect';
import { addInstance, getInstance, deleteInstance, getConfig } from './Instance';
import { doingString, addString, deleteString, getString } from './Redis';

export default function* index() {
    yield [
        takeLatest(CONNECT_DONE, connectDB),
        takeLatest(DISCONNECT, disconnectDB),
        takeLatest(INSTANCE_ADD, addInstance),
        takeLatest(INSTANCE_DELETE, deleteInstance),
        takeLatest(INSTANCE_GET, getInstance),
        takeLatest(STRING_DOING, doingString),
        takeLatest(STRING_ADD, addString),
        takeLatest(STRING_DELETE, deleteString),
        takeLatest(STRING_GET, getString)
    ]
}