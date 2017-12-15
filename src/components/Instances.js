import React from 'react';
import { connect } from 'react-redux';
import { Button } from 'antd';
import { actionConnect } from '../redux/actions/Connect';

class Instance extends React.PureComponent {
    constructor(props) {
        super(props)

    }

    connectToDB() {
        console.log('instances');
        this.props.actionConnect();
    }

    render() {
        return (
            <div>
                <Button type="primary" onClick={() => this.connectToDB()}>Test Connection</Button>
            </div>
        )
    }
}

function mapStateToProps(state) {
    return {}
}

export default connect(mapStateToProps, {
    actionConnect
})(Instance);