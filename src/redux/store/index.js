import { configureStore } from '@reduxjs/toolkit';
import reducer from '../reducer';

export default function configureStoreWrapper(initialState) {
    return configureStore({
        reducer,
        preloadedState: initialState,
        devTools: true,
    });
}