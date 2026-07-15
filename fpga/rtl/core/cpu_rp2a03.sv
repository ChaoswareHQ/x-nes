module cpu_rp2a03 (
    input  logic        clk,
    input  logic        rst_n,
    output logic [15:0] addr,
    input  logic [7:0]  data_in,
    output logic [7:0]  data_out,
    output logic        we
);

    logic [7:0]  A, X, Y, SP, P;
    logic [15:0] PC;

    logic [7:0]  opcode;        // Current instruction
    logic [2:0]  cycle;         // 0..7 cycle counter
    logic [15:0] addr_operand;  // Effective address (calculated early)
    logic [7:0]  operand_data;  // Data fetched from addr_operand
    logic        is_write;      // Does this instruction write to memory?
    logic [7:0]  alu_out;       // ALU result

    always_comb begin
        addr_operand = PC;      // Default (fallback)
        unique case (opcode)
            8'hA9, 8'hA2, 8'hA0, 8'h69, 8'hE9, 8'h29, 8'h09, 8'h49: 
                addr_operand = PC + 16'b1;

            8'hA5, 8'h85, 8'hA6, 8'h86, 8'hA4, 8'h84: 
                addr_operand = {8'h00, data_in}; // data_in from current fetch

            8'hAD, 8'h8D, 8'hAE, 8'h8E, 8'hAC, 8'h8C:
                addr_operand = data_in + (PC[15:8] << 8); // Simplified

            default: addr_operand = PC;
        endcase
    end

    always_comb begin
        alu_out = A; // Default
        unique case (opcode)
            8'hA9, 8'hA5, 8'hAD: alu_out = operand_data; // LDA
            8'hA2, 8'hA6, 8'hAE: alu_out = operand_data; // LDX
            8'hA0, 8'hA4, 8'hAC: alu_out = operand_data; // LDY

            8'h69, 8'h65, 8'h6D: begin
                logic [8:0] sum;
                sum = A + operand_data + {7'b0, P[0]};
                alu_out = sum[7:0];
            end

            8'h29, 8'h25, 8'h2D: alu_out = A & operand_data;
            8'h09, 8'h05, 8'h0D: alu_out = A | operand_data;
            8'h49, 8'h45, 8'h4D: alu_out = A ^ operand_data;

            8'hE8: alu_out = X + 1; // INX
            8'hCA: alu_out = X - 1; // DEX
            8'hC8: alu_out = Y + 1; // INY
            8'h88: alu_out = Y - 1; // DEY

            default: alu_out = A;
        endcase
    end

    typedef enum logic [1:0] { IDLE, FETCH, EXEC } state_t;
    state_t current_state, next_state;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            current_state <= IDLE;
            PC <= 16'hFFFC;      // 6502 reset vector
            A <= 8'h00; X <= 8'h00; Y <= 8'h00;
            SP <= 8'hFD;
            P <= 8'b0010_0100;   // Interrupts disabled
            cycle <= 3'b0;
            opcode <= 8'h00;
            we <= 1'b0;
        end else begin
            current_state <= next_state;
            cycle <= cycle + 1;

            case (current_state)
                IDLE: begin
                    addr <= PC;
                    we <= 1'b0;
                    next_state <= FETCH;
                end

                FETCH: begin
                    opcode <= data_in;
                    PC <= PC + 1;  // Advance program counter
                    addr <= PC + 1; // Pre-fetch the next byte (operand)
                    
                    if (opcode == 8'hEA || opcode == 8'h60) begin // NOP / RTS
                        next_state <= EXEC;
                        cycle <= 3'b0; // Reset cycle counter for next instr
                    end else begin
                        next_state <= EXEC;
                    end
                end

                EXEC: begin
                    unique case (opcode)
                        8'hA9, 8'hA5, 8'hAD, 8'h69, 8'h65, 8'h6D,
                        8'h29, 8'h25, 8'h2D, 8'h09, 8'h05, 8'h0D,
                        8'h49, 8'h45, 8'h4D: 
                            A <= alu_out;

                        8'hA2, 8'hA6, 8'hAE, 8'hE8, 8'hCA:
                            X <= alu_out;

                        8'hA0, 8'hA4, 8'hAC, 8'hC8, 8'h88:
                            Y <= alu_out;

                        8'h85, 8'h8D: begin
                            we <= 1'b1;
                            addr <= addr_operand;
                            data_out <= A;
                        end

                        8'h86, 8'h8E: begin
                            we <= 1'b1;
                            addr <= addr_operand;
                            data_out <= X;
                        end

                        8'h84, 8'h8C: begin
                            we <= 1'b1;
                            addr <= addr_operand;
                            data_out <= Y;
                        end
                    endcase

                    if (opcode inside {8'hA9, 8'hA5, 8'hAD, 8'h69, 8'h65, 8'h6D}) begin
                        P[7] <= alu_out[7];            // Negative
                        P[1] <= (alu_out == 8'b0);     // Zero
                    end

                    next_state <= IDLE;
                    cycle <= 3'b0;
                end
            endcase
        end
    end

    always_comb begin
        if (we) begin
            data_out = A;
        end else begin
            data_out = 8'h00;
        end
    end

endmodule