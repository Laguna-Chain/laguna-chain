contract Basic {

    bool private value;

    constructor(bool init) {
        value = init;
    }

    function getValue() view public returns (bool) {
        return value;
    }

    function flip() public {
        value = !value;
    }

}