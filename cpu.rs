use std::collections::HashMap;
use crate::opcodes;

pub struct CPU {
    pub register_a: u8, //acumulador_a
    pub register_x: u8, //registrador_x
    pub register_y: u8, //registrador_y
    pub stack_pointer: u8, //ponteiro_pilha
    pub program_counter: u16, //contador_programa
    pub status: u8, //registrador_flags_status
    memory: [u8; 0xFFFF]
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
    Absolute_Y, // ex: Se Y vale 0A, a instrução LDA $2000,Y vai ler o endereço $200A.

    /*O conceito de "indireto" significa usar a memória para guardar um ponteiro (um endereço que aponta para outro endereço). 
    No 6502, esses modos usam a Página Zero para buscar o endereço de 16 bits final. */

    /*O processador pega o endereço da Página Zero fornecido, soma o valor de X a ele e, LDA ($10),Y. O processador vai no endereço $0010 da Página Zero e lê o endereço de 16 bits salvo ali (ex: $4000). Depois, ele pega esse $4000 e soma o valor de Y (se Y for 05, o destino final será $4005). É ideal para navegar por matrizes e arrays de dados na memória.
    no endereço resultante, encontra os 2 bytes que formam o endereço final onde o dado realmente está. */
    Indirect_X, // ex :LDA ($10,X). Se X é 02, o processador vai até a Página Zero no endereço $0012 e lê o endereço de 16 bits que está guardado ali. Digamos que ali dentro esteja escrito $3000. O processador então vai até a memória $3000 buscar o dado.
    /*É o modo indireto mais comum e útil do 6502. O processador vai até o endereço da Página Zero fornecido para ler um endereço de 16 bits armazenado lá. 
    Depois que ele pega esse endereço, ele soma o valor do registrador Y a ele para chegar ao destino final.*/
    Indirect_Y, // ex: LDA ($10),Y. O processador vai no endereço $0010 da Página Zero e lê o endereço de 16 bits salvo ali (ex: $4000). Depois, ele pega esse $4000 e soma o valor de Y (se Y for 05, o destino final será $4005). É ideal para navegar por matrizes e arrays de dados na memória.

    /*A instrução não precisa de um endereço de memória explícito.*/
    NoneAddressing, //ex: INX
}

trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
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
            stack_pointer: 0,
            program_counter: 0,
            status: 0,
            memory: [0; 0xFFFF]
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,

            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }

            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }

            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),

            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }

            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.program_counter);
                // Correção: usando register_y aqui
                let addr = base.wrapping_add(self.register_y as u16);
                addr
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
                let hi = self.mem_read(base.wrapping_add(1) as u16); // Removido o ;; duplicado
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }

            AddressingMode::Relative => {
                let offset = self.mem_read(self.program_counter) as i8;
                
                let base = self.program_counter.wrapping_add(1);
                
                if offset >= 0 {
                    base.wrapping_add(offset as u16)
                } else {
                    base.wrapping_sub((-offset) as u16)
                }
            }

            AddressingMode::NoneAddressing => {
                panic!("modo {:?} não é reconhecido", mode);
            }
        }
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status |= 0b0000_0010;
        } else {
            self.status &= 0b1111_1101;
        }

        if result & 0b1000_0000 != 0 {
            self.status |= 0b1000_0000;
        } else {
            self.status &= 0b0111_1111;
        }
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }
    
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run()
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = 0;
        self.status = 0;
        
        self.program_counter = self.mem_write_u16(0xFFFC);
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            let opcode = opcodes.get(&code).expect(&format!("OpCode {:?} não é reconhecido", code));

            match code {
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                    self.lda(&opcode.mode);
                }

                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                }

                0xAA => self.tax(),
                0xe8 => self.inx(),
                0x00 => return,
                _ => todo!(),
            }

            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }
}
