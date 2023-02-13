int fib(int n) {
    if (n == 0 || n == 1) {
        return n;
    } else {
        return fib(n-1) + fib(n-2);
    }
}

int main() {
    if (fib(5) != 5) {
        return 1;
    }
    return 0;
}
