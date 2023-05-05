#ifndef CKB_VM_BPFC_RISCV_H_
#define CKB_VM_BPFC_RISCV_H_

// Start of output from running cbindgen -l c in ckb-vm/definitions
#include <stdarg.h>
// #include <stdbool.h>
// #include <stdint.h>
// #include <stdlib.h>

#define RISCV_PAGE_SHIFTS 12

#define RISCV_PAGESIZE (1 << RISCV_PAGE_SHIFTS)

#define RISCV_GENERAL_REGISTER_NUMBER 32

#define RISCV_MAX_MEMORY (4 << 20)

#define DEFAULT_STACK_SIZE (1 << 20)

#define RISCV_PAGES (RISCV_MAX_MEMORY / RISCV_PAGESIZE)

#define MEMORY_FRAME_SHIFTS 18

#define MEMORY_FRAMESIZE (1 << MEMORY_FRAME_SHIFTS)

#define MEMORY_FRAMES (RISCV_MAX_MEMORY / MEMORY_FRAMESIZE)

#define MEMORY_FRAME_PAGE_SHIFTS (MEMORY_FRAME_SHIFTS - RISCV_PAGE_SHIFTS)

#define ISA_IMC 0

#define ISA_B 1

#define ISA_MOP 2

#define ISA_A 4

#define TRACE_SIZE 8192

#define TRACE_ITEM_LENGTH 16

#define RET_DECODE_TRACE 1

#define RET_ECALL 2

#define RET_EBREAK 3

#define RET_DYNAMIC_JUMP 4

#define RET_MAX_CYCLES_EXCEEDED 5

#define RET_CYCLES_OVERFLOW 6

#define RET_OUT_OF_BOUND 7

#define RET_INVALID_PERMISSION 8

#define RET_SLOWPATH 9

#define FLAG_FREEZED 1

#define FLAG_EXECUTABLE 2

#define FLAG_WXORX_BIT 2

#define FLAG_WRITABLE (~FLAG_EXECUTABLE & FLAG_WXORX_BIT)

#define FLAG_DIRTY 4

#define ZERO 0

#define RA 1

#define SP 2

#define GP 3

#define FP 8

#define TP 4

#define T0 5

#define T1 6

#define T2 7

#define T3 28

#define T4 29

#define T5 30

#define T6 31

#define S0 8

#define S1 9

#define S2 18

#define S3 19

#define S4 20

#define S5 21

#define S6 22

#define S7 23

#define S8 24

#define S9 25

#define S10 26

#define S11 27

#define A0 10

#define A1 11

#define A2 12

#define A3 13

#define A4 14

#define A5 15

#define A6 16

#define A7 17

typedef uint16_t InstructionOpcode;

#define OP_UNLOADED 16

#define OP_ADD 17

#define OP_ADDI 18

#define OP_ADDIW 19

#define OP_ADDW 20

#define OP_AND 21

#define OP_ANDI 22

#define OP_AUIPC 23

#define OP_BEQ 24

#define OP_BGE 25

#define OP_BGEU 26

#define OP_BLT 27

#define OP_BLTU 28

#define OP_BNE 29

#define OP_DIV 30

#define OP_DIVU 31

#define OP_DIVUW 32

#define OP_DIVW 33

#define OP_EBREAK 34

#define OP_ECALL 35

#define OP_FENCE 36

#define OP_FENCEI 37

#define OP_JAL 38

#define OP_JALR_VERSION0 39

#define OP_JALR_VERSION1 40

#define OP_LB_VERSION0 41

#define OP_LB_VERSION1 42

#define OP_LBU_VERSION0 43

#define OP_LBU_VERSION1 44

#define OP_LD_VERSION0 45

#define OP_LD_VERSION1 46

#define OP_LH_VERSION0 47

#define OP_LH_VERSION1 48

#define OP_LHU_VERSION0 49

#define OP_LHU_VERSION1 50

#define OP_LUI 51

#define OP_LW_VERSION0 52

#define OP_LW_VERSION1 53

#define OP_LWU_VERSION0 54

#define OP_LWU_VERSION1 55

#define OP_MUL 56

#define OP_MULH 57

#define OP_MULHSU 58

#define OP_MULHU 59

#define OP_MULW 60

#define OP_OR 61

#define OP_ORI 62

#define OP_REM 63

#define OP_REMU 64

#define OP_REMUW 65

#define OP_REMW 66

#define OP_SB 67

#define OP_SD 68

#define OP_SH 69

#define OP_SLL 70

#define OP_SLLI 71

#define OP_SLLIW 72

#define OP_SLLW 73

#define OP_SLT 74

#define OP_SLTI 75

#define OP_SLTIU 76

#define OP_SLTU 77

#define OP_SRA 78

#define OP_SRAI 79

#define OP_SRAIW 80

#define OP_SRAW 81

#define OP_SRL 82

#define OP_SRLI 83

#define OP_SRLIW 84

#define OP_SRLW 85

#define OP_SUB 86

#define OP_SUBW 87

#define OP_SW 88

#define OP_XOR 89

#define OP_XORI 90

#define OP_LR_W 91

#define OP_SC_W 92

#define OP_AMOSWAP_W 93

#define OP_AMOADD_W 94

#define OP_AMOXOR_W 95

#define OP_AMOAND_W 96

#define OP_AMOOR_W 97

#define OP_AMOMIN_W 98

#define OP_AMOMAX_W 99

#define OP_AMOMINU_W 100

#define OP_AMOMAXU_W 101

#define OP_LR_D 102

#define OP_SC_D 103

#define OP_AMOSWAP_D 104

#define OP_AMOADD_D 105

#define OP_AMOXOR_D 106

#define OP_AMOAND_D 107

#define OP_AMOOR_D 108

#define OP_AMOMIN_D 109

#define OP_AMOMAX_D 110

#define OP_AMOMINU_D 111

#define OP_AMOMAXU_D 112

#define OP_ADDUW 113

#define OP_ANDN 114

#define OP_BCLR 115

#define OP_BCLRI 116

#define OP_BEXT 117

#define OP_BEXTI 118

#define OP_BINV 119

#define OP_BINVI 120

#define OP_BSET 121

#define OP_BSETI 122

#define OP_CLMUL 123

#define OP_CLMULH 124

#define OP_CLMULR 125

#define OP_CLZ 126

#define OP_CLZW 127

#define OP_CPOP 128

#define OP_CPOPW 129

#define OP_CTZ 130

#define OP_CTZW 131

#define OP_MAX 132

#define OP_MAXU 133

#define OP_MIN 134

#define OP_MINU 135

#define OP_ORCB 136

#define OP_ORN 137

#define OP_REV8 138

#define OP_ROL 139

#define OP_ROLW 140

#define OP_ROR 141

#define OP_RORI 142

#define OP_RORIW 143

#define OP_RORW 144

#define OP_SEXTB 145

#define OP_SEXTH 146

#define OP_SH1ADD 147

#define OP_SH1ADDUW 148

#define OP_SH2ADD 149

#define OP_SH2ADDUW 150

#define OP_SH3ADD 151

#define OP_SH3ADDUW 152

#define OP_SLLIUW 153

#define OP_XNOR 154

#define OP_ZEXTH 155

#define OP_WIDE_MUL 156

#define OP_WIDE_MULU 157

#define OP_WIDE_MULSU 158

#define OP_WIDE_DIV 159

#define OP_WIDE_DIVU 160

#define OP_FAR_JUMP_REL 161

#define OP_FAR_JUMP_ABS 162

#define OP_ADC 163

#define OP_SBB 164

#define OP_ADCS 165

#define OP_SBBS 166

#define OP_ADD3A 167

#define OP_ADD3B 168

#define OP_ADD3C 169

#define OP_CUSTOM_LOAD_UIMM 170

#define OP_CUSTOM_LOAD_IMM 171

#define OP_CUSTOM_TRACE_END 172

#define MINIMAL_OPCODE OP_UNLOADED

#define MAXIMUM_OPCODE OP_CUSTOM_TRACE_END

// End of output from running cbindgen -l c in ckb-vm/definitions

typedef uint64_t Instruction;
typedef uint RegisterIndex;
typedef int32_t SImmediate;
typedef uint32_t UImmediate;

#define EXTRACT_OPCODE(i) (((i >> 8) & 0xff00) | (i & 0x00ff))
#define INSTRUCTION_LENGTH(i) (((i >> 24) & 0x0f) << 1)

#define RTYPE_OP(i) ((i >> 16 << 8) | (i & 0xFF))
#define RTYPE_RD(i) (i >> 8)
#define RTYPE_RS1(i) (i >> 32)
#define RTYPE_RS2(i) (i >> 40)

#define ITYPE_OP(i) ((i >> 16 << 8) | (i & 0xFF))
#define ITYPE_RD(i) (i >> 8)
#define ITYPE_RS1(i) (i >> 32)
#define ITYPE_IMMEDIATE_U(i) (i >> 40)
#define ITYPE_IMMEDIATE_S(i) (i >> 40)

#define STYPE_OP(i) ((i >> 16 << 8) | (i & 0xFF))
#define STYPE_RS1(i) (i >> 32)
#define STYPE_RS2(i) (i >> 8)
#define STYPE_IMMEDIATE_U(i) (i >> 40)
#define STYPE_IMMEDIATE_S(i) (i >> 40)

#define UTYPE_OP(i) ((i >> 16 << 8) | (i & 0xFF))
#define UTYPE_RD(i) (i >> 8)
#define UTYPE_IMMEDIATE_U(i) (i >> 32)
#define UTYPE_IMMEDIATE_S(i) (i >> 32)

#define R4TYPE_OP(i) ((i >> 16 << 8) | (i & 0xFF))
#define R4TYPE_RD(i) (i >> 8)
#define R4TYPE_RS1(i) (i >> 32)
#define R4TYPE_RS2(i) (i >> 40)
#define R4TYPE_RS3(i) (i >> 48)

#define R5TYPE_OP(i) ((i >> 16 << 8) | (i & 0xFF))
#define R5TYPE_RD(i) (i >> 8)
#define R5TYPE_RS1(i) (i >> 32)
#define R5TYPE_RS2(i) (i >> 40)
#define R5TYPE_RS3(i) (i >> 48)
#define R5TYPE_RS4(i) (i >> 56)

#define X0 0
#define X1 1
#define X2 2
#define X3 3
#define X4 4
#define X5 5
#define X6 6
#define X7 7
#define X8 8
#define X9 9
#define X10 10
#define X11 11
#define X12 12
#define X13 13
#define X14 14
#define X15 15
#define X16 16
#define X17 17
#define X18 18
#define X19 19
#define X20 20
#define X21 21
#define X22 22
#define X23 23
#define X24 24
#define X25 25
#define X26 26
#define X27 27
#define X28 28
#define X29 29
#define X30 30
#define X31 31

#endif /* CKB_VM_BPFC_RISCV_H_ */
