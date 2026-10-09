use std::collections::HashMap;
use crate::opcodes;
use bitflags::bitflags;

bitflags! {
    /// # Status Register (P) https://www.nesdev.org/wiki/Status_flags
    ///
    ///   7 6 5 4 3 2 1 0
    ///   N V _ B D I Z C
    ///   | |   | | | | +--- Carry Flag
    ///   | |   | | | +----- Zero Flag
    ///   | |   | | +------- Interrupt Disable
    ///   | |   | +--------- Decimal Mode (não utilizado no NES)
    ///   | |   +----------- Break Command
    ///   | +--------------- Overflow Flag
    ///   +----------------- Negative Flag
    ///
    
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const BREAK2            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub struct CPU {
    pub register_a: u8,        //acumulador_a
    pub register_x: u8,        //registrador_x
    pub register_y: u8,        //registrador_y
    pub stack_pointer: u8,     //ponteiro_pilha
    pub program_counter: u16,  //contador_programa
    pub status: CpuFlags,      //registrador_flags_status
    memory: [u8; 0x10000],
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    /*O dado não é um endereço de memória, mas sim o próprio valor numérico constante que a instrução vai usar imediatamente.
    No código, é identificado pelo símbolo # */
    Immediate, //ex: LDA #$05
    
    /*A "Página Zero" corresponde aos primeiros 256 bytes da memória RAM (endereços de $0000 a $00FF). 
    Como esses endereços precisam de apenas 1 byte para serem representados (em vez de 2 bytes), as instruções que os utilizam são mais rápidas e ocupam menos espaço. */
    
    /*A instrução acessa diretamente um endereço localizado estritamente na Página Zero.*/
    ZeroPage, // ex: LDA $10 (Carrega o valor que estiver guardado na memória RAM no endereço $0010) 
    /*O endereço final é calculado somando o endereço base da Página Zero fornecido com o valor atual do registrador X. 
    Se a soma passar de $FF, ela dá a volta (wrap around) e continua no início da Página Zero.*/
    ZeroPage_X, // ex: Se X vale 02, a instrução LDA $10, X vai ler o endereço $0012.
    /*Funciona exatamente igual ao ZeroPage_X, mas utiliza o registrador Y. É usado por pouquíssimas instruções (como LDX e STX).*/
    ZeroPage_Y, // ex: Se Y vale 05, a instrução LDX $10, Y vai ler o endereço $0015.

    /*Os modos "Absolute" permitem acessar qualquer lugar de toda a memória do sistema (de $0000 até $FFFF), pois utilizam endereços completos de 2 bytes (16 bits). */
    
    /*A instrução aponta diretamente para um endereço fixo e completo na memória.*/
    Absolute, // ex: LDA $2000 (Carrega o valor armazenado na memória no endereço $2000).
    /*O endereço final é calculado somando o endereço absoluto de 16 bits fornecido com o valor atual do registrador X.*/
    Absolute_X, // ex: Se X vale 05, a instrução LDA $2000,X vai ler o endereço $2005.
    /*Funciona exatamente igual ao Absolute_X, mas soma o valor do registrador Y ao endereço absoluto fornecido.*/
    Absolute_Y, // ex: Se Y vale 0A, a instrução LDA $2000,Y vai ler o endereço $2000 + Y.

    /*Usado só pelo JMP: o operando é um ponteiro de 16 bits que guarda o endereço de destino.*/
    Indirect, //ex: JMP ($2000)

    /*O conceito de "indireto" significa usar a memória para guardar um ponteiro (um endereço que aponta para outro endereço). 
    No 6502, esses modos usam a Página Zero para buscar o endereço de 16 bits final. */
    Indirect_X, // ex :LDA ($10,X). Se X é 02, o processador vai até a Página Zero no endereço $0012 e lê o endereço de 16 bits que está guardado ali.
    Indirect_Y, // ex: LDA ($10),Y. O processador vai no endereço $0010 da Página Zero e lê o endereço de 16 bits salvo ali e soma Y.

    /*Usado por instruções de desvio (branches) para calcular saltos relativos baseados no PC.*/
    Relative,

    /*A instrução opera direto no registrador A, sem acessar a memória.*/
    Accumulator, //ex: ASL A

    /*A instrução não precisa de um endereço de memória explícito.*/
    NoneAddressing, //ex: INX
}

pub trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos.wrapping_add(1)) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos.wrapping_add(1), hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: STACK_RESET,
            program_counter: 0,
            status: CpuFlags::from_bits_truncate(0b100100),
            memory: [0; 0x10000],
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                pos.wrapping_add(self.register_x) as u16
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                pos.wrapping_add(self.register_y) as u16
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.program_counter);
                base.wrapping_add(self.register_x as u16)
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.program_counter);
                base.wrapping_add(self.register_y as u16)
            }
            AddressingMode::Indirect => {
                let mem_address = self.mem_read_u16(self.program_counter);
                // Bug de hardware do 6502: se o ponteiro está em $xxFF, o byte alto vem de $xx00
                if (mem_address & 0x00FF) == 0x00FF {
                    let lo = self.mem_read(mem_address);
                    let hi = self.mem_read(mem_address & 0xFF00);
                    (hi as u16) << 8 | (lo as u16)
                } else {
                    self.mem_read_u16(mem_address)
                }
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);
                let ptr: u8 = base.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read(base.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                deref_base.wrapping_add(self.register_y as u16)
            }
            AddressingMode::Relative => {
                let offset = self.mem_read(self.program_counter) as i8;
                let base = self.program_counter.wrapping_add(1);
                base.wrapping_add(offset as u16)
            }
            AddressingMode::Accumulator | AddressingMode::NoneAddressing => {
                panic!("modo {:?} não suportado", mode);
            }
        }
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        if (result & 0b1000_0000) != 0 {
            self.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.status.remove(CpuFlags::NEGATIVE);
        }
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.set_register_a(value);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.register_x = data;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.register_y = data;
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.set_register_a(data & self.register_a);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.set_register_a(data ^ self.register_a);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.set_register_a(data | self.register_a);
    }

    fn add_to_register_a(&mut self, data: u8) {
        let carry_in = self.status.contains(CpuFlags::CARRY) as u16;
        let sum = self.register_a as u16 + data as u16 + carry_in;

        self.status.set(CpuFlags::CARRY, sum > 0xff);

        let result = sum as u8;
        self.status.set(
            CpuFlags::OVERFLOW,
            (data ^ result) & (result ^ self.register_a) & 0x80 != 0,
        );

        self.set_register_a(result);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.add_to_register_a(data);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.add_to_register_a(!data);
    }

    fn asl_accumulator(&mut self) {
        let mut data = self.register_a;
        self.status.set(CpuFlags::CARRY, data >> 7 == 1);
        data <<= 1;
        self.set_register_a(data);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        self.status.set(CpuFlags::CARRY, data >> 7 == 1);
        data <<= 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn lsr_accumulator(&mut self) {
        let mut data = self.register_a;
        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        data >>= 1;
        self.set_register_a(data);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        data >>= 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn rol_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);
        self.status.set(CpuFlags::CARRY, data >> 7 == 1);
        data <<= 1;
        if old_carry {
            data |= 1;
        }
        self.set_register_a(data);
    }

    fn rol(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY);
        self.status.set(CpuFlags::CARRY, data >> 7 == 1);
        data <<= 1;
        if old_carry {
            data |= 1;
        }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn ror_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);
        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        data >>= 1;
        if old_carry {
            data |= 0b1000_0000;
        }
        self.set_register_a(data);
    }

    fn ror(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY);
        self.status.set(CpuFlags::CARRY, data & 1 == 1);
        data >>= 1;
        if old_carry {
            data |= 0b1000_0000;
        }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr).wrapping_add(1);
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr).wrapping_sub(1);
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn compare(&mut self, mode: &AddressingMode, compare_with: u8) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        if data <= compare_with {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }
        self.update_zero_and_negative_flags(compare_with.wrapping_sub(data));
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let and = self.register_a & data;
        if and == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }
        self.status.set(CpuFlags::NEGATIVE, (data & 0b10000000) != 0);
        self.status.set(CpuFlags::OVERFLOW, (data & 0b01000000) != 0);
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            self.program_counter = self.get_operand_address(&AddressingMode::Relative);
        }
    }

    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read((STACK as u16) + self.stack_pointer as u16)
    }

    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    fn php(&mut self) {
        let mut flags = self.status;
        flags.insert(CpuFlags::BREAK);
        flags.insert(CpuFlags::BREAK2);
        self.stack_push(flags.bits());
    }

    fn plp(&mut self) {
        self.status = CpuFlags::from_bits_truncate(self.stack_pop());
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x0600..(0x0600 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x0600);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = STACK_RESET;
        self.status = CpuFlags::from_bits_truncate(0b100100);
        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let opcodes: &HashMap<u8, &'static opcodes::OpCode> = &*opcodes::OPCODES_MAP;

        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter = self.program_counter.wrapping_add(1);
            let program_counter_state = self.program_counter;

            let opcode = opcodes
                .get(&code)
                .unwrap_or_else(|| panic!("OpCode {:#04X} não é reconhecido", code));

            match code {
                // LDA
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => self.lda(&opcode.mode),
                // LDX
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => self.ldx(&opcode.mode),
                // LDY
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => self.ldy(&opcode.mode),
                // STA
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                // STX
                0x86 | 0x96 | 0x8e => {
                    let addr = self.get_operand_address(&opcode.mode);
                    self.mem_write(addr, self.register_x);
                }
                // STY
                0x84 | 0x94 | 0x8c => {
                    let addr = self.get_operand_address(&opcode.mode);
                    self.mem_write(addr, self.register_y);
                }
                // Registers & Transfers
                0xAA => self.tax(),
                0xe8 => self.inx(),
                0xc8 => self.iny(),
                0xca => self.dex(),
                0x88 => self.dey(),
                0xa8 => { self.register_y = self.register_a; self.update_zero_and_negative_flags(self.register_y); }
                0x8a => { self.register_a = self.register_x; self.update_zero_and_negative_flags(self.register_a); }
                0x98 => { self.register_a = self.register_y; self.update_zero_and_negative_flags(self.register_a); }
                0xba => { self.register_x = self.stack_pointer; self.update_zero_and_negative_flags(self.register_x); }
                0x9a => { self.stack_pointer = self.register_x; }
                
                // Arithmetic
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => self.adc(&opcode.mode),
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => self.sbc(&opcode.mode),

                // Logic & Compare
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode),
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => self.eor(&opcode.mode),
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode),
                0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => self.compare(&opcode.mode, self.register_a),
                0xc0 | 0xc4 | 0xcc => self.compare(&opcode.mode, self.register_y),
                0xe0 | 0xe4 | 0xec => self.compare(&opcode.mode, self.register_x),
                0x24 | 0x2c => self.bit(&opcode.mode),

                // Shifts & Rotates
                0x0a => self.asl_accumulator(),
                0x06 | 0x16 | 0x0e | 0x1e => self.asl(&opcode.mode),
                0x4a => self.lsr_accumulator(),
                0x46 | 0x56 | 0x4e | 0x5e => self.lsr(&opcode.mode),
                0x2a => self.rol_accumulator(),
                0x26 | 0x36 | 0x2e | 0x3e => self.rol(&opcode.mode),
                0x6a => self.ror_accumulator(),
                0x66 | 0x76 | 0x6e | 0x7e => self.ror(&opcode.mode),

                // INC / DEC (memória)
                0xe6 | 0xf6 | 0xee | 0xfe => self.inc(&opcode.mode),
                0xc6 | 0xd6 | 0xce | 0xde => self.dec(&opcode.mode),

                // Flags
                0x18 => self.status.remove(CpuFlags::CARRY),             // CLC
                0x38 => self.status.insert(CpuFlags::CARRY),             // SEC
                0x58 => self.status.remove(CpuFlags::INTERRUPT_DISABLE), // CLI
                0x78 => self.status.insert(CpuFlags::INTERRUPT_DISABLE), // SEI
                0xd8 => self.status.remove(CpuFlags::DECIMAL_MODE),      // CLD
                0xf8 => self.status.insert(CpuFlags::DECIMAL_MODE),      // SED
                0xb8 => self.status.remove(CpuFlags::OVERFLOW),          // CLV

                // Jumps & Subroutines
                0x4c | 0x6c => {
                    self.program_counter = self.get_operand_address(&opcode.mode);
                }
                0x20 => {
                    // empilha o endereço do último byte da instrução (pc + 2 - 1 == pc + 1)
                    self.stack_push_u16(self.program_counter.wrapping_add(1));
                    self.program_counter = self.get_operand_address(&opcode.mode);
                }
                0x60 => {
                    self.program_counter = self.stack_pop_u16().wrapping_add(1);
                }
                // RTI: puxa o status e depois o PC (sem somar 1, diferente do RTS)
                0x40 => {
                    self.plp();
                    self.program_counter = self.stack_pop_u16();
                }

                // Branches
                0xf0 => self.branch(self.status.contains(CpuFlags::ZERO)),
                0xd0 => self.branch(!self.status.contains(CpuFlags::ZERO)),
                0xb0 => self.branch(self.status.contains(CpuFlags::CARRY)),
                0x90 => self.branch(!self.status.contains(CpuFlags::CARRY)),
                0x30 => self.branch(self.status.contains(CpuFlags::NEGATIVE)),
                0x10 => self.branch(!self.status.contains(CpuFlags::NEGATIVE)),
                0x70 => self.branch(self.status.contains(CpuFlags::OVERFLOW)),
                0x50 => self.branch(!self.status.contains(CpuFlags::OVERFLOW)),

                // Stack push/pop flags or A
                0x48 => self.stack_push(self.register_a),
                0x68 => { let data = self.stack_pop(); self.set_register_a(data); }
                0x08 => self.php(),
                0x28 => self.plp(),

                0xea => {} // NOP
                // BRK: por enquanto encerra o loop (útil para testes).
                // Quando houver interrupções, deve empilhar PC e status e pular para o vetor em $FFFE.
                0x00 => return,
                _ => todo!("Opcode 0x{:02X} não implementado", code),
            }

            if program_counter_state == self.program_counter {
                self.program_counter = self.program_counter.wrapping_add((opcode.len - 1) as u16);
            }

            callback(self);
        }
    }
}