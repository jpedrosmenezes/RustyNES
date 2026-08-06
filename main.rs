pub struct CPU {
    pub register_a: u8, //acumulador_a
    pub register_x: u8, //registrador_x
    pub register_y: u8, //registrador_y
    pub stack_pointer: u8, //ponteiro_pilha
    pub program_counter: u16, //contador_programa
    pub status: u8, //registrador_flags_status
    memory: [u8; 0xFFFF]
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
    fn lda(&mut self, value: u8) {
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


    
    pub fn interpret(&mut self, program: Vec<u8>) {
        self.program_counter = 0;

        loop {
            if self.program_counter as usize >= program.len() {
                return;
            }
            
            let opscode =   program[self.program_counter as usize];
            self.program_counter += 1;
        
            match opscode {
                0xA9 => {
                    let param = program[self.program_counter as usize];
                    self.program_counter +=1;
                    self.lda(param);
                }

                0xAA => self.tax(),

                0xE8 => self.inx(),

                0x00 => return,

                _ => todo!(),
            }
        }
    }
}
fn main() {
    // 1. Inicializa a CPU
    let mut cpu = CPU::new();

    // 2. Cria um "programa" em bytecode
    // 0xA9 0x05 -> LDA #05 (Carrega 5 no acumulador A)
    // 0xAA      -> TAX    (Transfere A para o registrador X)
    // 0xE8      -> INX    (Incrementa X para 6)
    // 0x00      -> BRK    (Para a execução)
    let program = vec![0xA9, 0x05, 0xAA, 0xE8, 0x00];

    // 3. Roda o interpretador
    cpu.interpret(program);

    // 4. Verifica os resultados
    println!("--- Teste da CPU ---");
    println!("Registrador A: {}", cpu.register_a);
    println!("Registrador X: {}", cpu.register_x);
    println!("Status (Flags): {:08b}", cpu.status);
    
    // Verificação simples
    assert_eq!(cpu.register_x, 0x06);
    println!("Teste concluído com sucesso!");
}
