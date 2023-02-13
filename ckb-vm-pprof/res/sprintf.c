#include <stdio.h>

static char buffer[8];

int main() {
    return sprintf(buffer, "%02x", 42);
}
