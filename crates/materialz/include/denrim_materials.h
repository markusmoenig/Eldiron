#ifndef DENRIM_MATERIALS_H
#define DENRIM_MATERIALS_H
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct DmProgram DmProgram;
typedef struct DmShader DmShader;

/* Strings are UTF-8 and NUL terminated. Returned programs/strings are owned.
 * error may be NULL; otherwise it must point to writable pointer storage.
 * Each operation initializes *error to NULL and assigns an owned diagnostic
 * on failure. Free a previous diagnostic before reusing that pointer storage.
 * Callers must keep handles alive for the duration of operations. A program
 * can be sampled/generated from multiple threads. Mutating or freeing a handle
 * requires exclusive access, synchronized with all readers. Shader handles own
 * their data and do not borrow program storage. No native pointers are serialized.
 */
DmProgram *dm_compile(const char *source, char **error);
void dm_program_free(DmProgram *program);
void dm_string_free(char *string);

/* channel: "base color", "roughness", "metallic", "height", "opacity", "distance". */
char *dm_metal(const DmProgram *program, const char *material,
               const char *channel, char **error);

/* output must have capacity for 3 floats. Returns 1 or 3, or -1 on failure.
 * Values are linear and unclamped. On failure output is unchanged. */
int dm_sample(const DmProgram *program, const char *material,
              const char *channel, float u, float v,
              float *output, char **error);

/* Owned versioned JSON catalog: source names, groups and parameter definitions. */
char *dm_catalog_json(char **error);
/* Shared BSDF/geometry channel types, defaults, groups and renderer field IDs. */
char *dm_bsdf_catalog_json(char **error);
/* Owned canonical project text, including committed parameter edits. */
char *dm_project_text(const DmProgram *program, char **error);
/* Structured authoring model with resolved per-tile distance capability. */
char *dm_project_json(const DmProgram *program, char **error);
/* Selected tile definition and language as JSON. Built-in definitions are read-only. */
char *dm_tile_source(const DmProgram *program, const char *material, const char *row,
                     size_t tile, char **error);
/* Zero-based tile index. Returns 1 or 0. Validates before committing. A connected
 * parameter cannot be silently replaced with a constant. Needs exclusive access. */
int dm_set_constant(DmProgram *program, const char *material, const char *row,
                    size_t tile, const char *parameter, const float *values,
                    size_t count, char **error);

/* Live Metal path: output float4 buffer(0), uint2 dimensions buffer(1), and
 * constant float4 parameter array buffer(2). Compile source once, upload changed
 * parameter values after each successful refresh. No handle owns a GPU object. */
DmShader *dm_shader_compile(const DmProgram *program, const char *material,
                            const char *channel, char **error);
void dm_shader_free(DmShader *shader);
/* Execute material_cache for each prerequisite in order into a read/write RGBA32F
 * texture2d_array at texture(0), two slices per pass (premultiplied RGBA, sampled distance).
 * Same combined parameter buffer as the final shader. Cache region at buffer(4).
 * Grow the requested region by the returned halo while retaining pixel spacing. */
size_t dm_shader_pass_count(const DmShader *shader);
char *dm_shader_pass_source(const DmShader *shader,size_t index,char **error);
char *dm_shader_pass_key(const DmShader *shader,size_t index,char **error);
float dm_shader_pixel_padding(const DmShader *shader);
float dm_shader_uv_padding(const DmShader *shader);
size_t dm_shader_fixed_domain_count(const DmShader *shader);
int dm_shader_fixed_domain(const DmShader *shader,size_t index,float *output,char **error);
int dm_shader_domain_is_fixed(const DmShader *shader,size_t index);
size_t dm_shader_domain_count(const DmShader *shader);
int dm_shader_domain(const DmShader *shader,size_t index,float *output,char **error);
char *dm_shader_source(const DmShader *shader, char **error);
char *dm_shader_layout_json(const DmShader *shader, char **error);
/* Includes padding: every parameter uses one 16-byte float4, even a scalar/bool.
 * Returns zero for a null handle. */
size_t dm_shader_float_count(const DmShader *shader);
/* capacity is in floats. Returns floats copied or -1; errors leave output intact. */
int dm_shader_copy_parameters(const DmShader *shader, float *output,
                              size_t capacity, char **error);
/* Returns 1 or 0. Structural changes require a new shader; failure retains old
 * data. Needs exclusive access to the shader and shared access to the program. */
int dm_shader_refresh(DmShader *shader, const DmProgram *program, char **error);

/* output has 5 floats: straight-alpha RGBA, distance. Returns 1 for an SDF,
 * 0 for image-only (distance=0), -1 on error. Failure leaves output untouched.
 * pixel_width is in document units (1/min(width,height) for previews). */
int dm_tile_sample(const DmProgram *program, const char *material, const char *row,
                   size_t tile, float u, float v, float pixel_width,
                   float *output, char **error);
/* Same shader/buffer lifecycle as dm_shader_compile, returning RGBA thumbnails.
 * mode: "image", "shape", "distance". Every tile supports image mode. */
DmShader *dm_tile_shader_compile(const DmProgram *program, const char *material,
                                 const char *row, size_t tile, const char *mode,
                                 char **error);

#ifdef __cplusplus
}
#endif
#endif
