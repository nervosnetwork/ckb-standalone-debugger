// uncomment to enable printf in CKB-VM
#define CKB_C_STDLIB_PRINTF
//#define CKB_C_STDLIB_PRINTF_BUFFER_SIZE 1024

#include "ckb_syscalls.h"
#include "stdio.h"
#include "stdlib.h"

int fib(int n) {
    if (n == 0 || n == 1) {
        printf("n = %d", n);
        return n;
    } else {
        printf("n = %d", n);
        return fib(n - 1) + fib(n - 2);
    }
}

int main() {
    if (fib(5) != 5) {
        return 1;
    }
    return 0;
}
