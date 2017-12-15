import React from 'react';
import ReactDOM from 'react-dom';

class MainWindow extends React.Component {
    render() {
        return (
            <div>main page!shit page</div>
        )
    }
}

ReactDOM.render(<MainWindow />, document.getElementById('content'))