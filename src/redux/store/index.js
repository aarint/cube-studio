import { createStore, applyMiddleware } from 'redux';
import thunkMiddleware from 'redux-thunk';
import reducer from '../reducer';

export default function configureStore(initialState) {
    /* eslint-disable no-underscore-dangle */
    return createStore(
        reducer,
        initialState,
        applyMiddleware(thunkMiddleware),
        window.__REDUX_DEVTOOLS_EXTENSION__ && window.__REDUX_DEVTOOLS_EXTENSION__()
    )
    /* eslint-enable */
}