int main() {
  _sbrk(1024*1024*2);

  for (int i = 0; i < 65536 * 2; i++) {
      __asm__("li a0, 16\r\n"
            "sub sp, sp, a0"
            :
            :
            :"a0"
            );
  }
  for (int i = 0; i < 65536 * 2; i++) {
      __asm__("li a0, 16\r\n"
            "add sp, sp, a0"
            :
            :
            :"a0"
            );
  }

  return 0;
}
