const int n = 2;

int a() {

    __asm__("li a1, 0x400000\r\n"
            "ld a0, 0(a1)"
            :
            :
            :"a0", "a1"
            );
    return n;
}

int b() {
    return a() + n;
}

int c() {
    return b() + n;
}

int main() {
    return c();
}
