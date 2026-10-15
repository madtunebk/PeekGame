pub fn split(input: &[u16]) -> Vec<Vec<u16>> {
    let mut result = Vec::new();
    let mut token = Vec::new();
    let mut quoted = false;
    let mut slashes = 0;
    let mut started = false;
    for &c in input {
        if c == 92 {
            slashes += 1;
            started = true;
            continue;
        }
        if c == 34 {
            token.extend(std::iter::repeat_n(92, slashes / 2));
            if slashes % 2 == 1 {
                token.push(34);
            } else {
                quoted = !quoted;
            }
            slashes = 0;
            started = true;
            continue;
        }
        token.extend(std::iter::repeat_n(92, slashes));
        slashes = 0;
        if (c == 32 || c == 9) && !quoted {
            if started {
                result.push(std::mem::take(&mut token));
                started = false;
            }
        } else {
            token.push(c);
            started = true;
        }
    }
    token.extend(std::iter::repeat_n(92, slashes));
    if started {
        result.push(token);
    }
    result
}
