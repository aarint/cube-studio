import { takeLatest } from 'redux-saga/effects';

import { ACTION_CONNECT, ACTION_DISCONNECT, ACTION_UPDATE_CONNECT_STATUS } from "../actions/Connect";

import { connectToDB } from './Connect';

export default function* index() {
    yield [
        takeLatest(ACTION_CONNECT, connectToDB)
    ]
}